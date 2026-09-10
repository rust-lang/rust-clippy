use crate::doc::{Fragments, MANUAL_INTRA_DOC_LINKS};
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_crate_store::ExternCrateSource;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::def::DefKind;
use rustc_hir::definitions::DefPathData;
use rustc_lint::LateContext;
use rustc_resolve::rustdoc::pulldown_cmark::LinkType;
use rustc_span::def_id::LocalDefId;
use rustc_span::{Symbol, kw};
use std::ops::Range;

#[expect(clippy::too_many_arguments)]
pub(crate) fn check(
    current_item: LocalDefId,
    dest_url: &str,
    doc: &str,
    doc_span: Range<usize>,
    fragments: &Fragments<'_>,
    reference_definitions: &FxHashMap<Box<str>, Range<usize>>,
    link_type: LinkType,
    link_id: &str,
    cx: &LateContext<'_>,
) {
    let Some(intra_doc_link) = check_docsrs(dest_url, cx).or_else(|| check_relative(current_item, dest_url, cx)) else {
        return;
    };
    let Some(span) = fragments.span(cx, doc_span.clone()) else {
        return;
    };
    span_lint_and_then(cx, MANUAL_INTRA_DOC_LINKS, span, "manual intra-doc link", |diag| {
        if matches!(
            link_type,
            LinkType::Reference
                | LinkType::ReferenceUnknown
                | LinkType::Collapsed
                | LinkType::CollapsedUnknown
                | LinkType::Shortcut
                | LinkType::ShortcutUnknown
        ) && let Some(refdef) = reference_definitions.get(link_id)
        {
            let Some(span) = fragments.span(cx, refdef.clone()) else {
                return;
            };
            let text = &doc[refdef.clone()];
            let text = &text[text.find('[').expect("markdown links use square brackets") + 1
                ..text.rfind(']').expect("markdown links use square brackets")];
            diag.span_suggestion_verbose(
                span,
                "consider linking by path instead",
                format!("[{text}]: {intra_doc_link}"),
                Applicability::MachineApplicable,
            );
        } else if matches!(link_type, LinkType::Inline) {
            let text = &doc[doc_span];
            let text = &text[text.find('[').expect("markdown links use square brackets") + 1
                ..text.rfind(']').expect("markdown links use square brackets")];
            diag.span_suggestion_verbose(
                span,
                "consider linking by path instead",
                format!("[{text}]({intra_doc_link})"),
                Applicability::MachineApplicable,
            );
        } else {
            // used by <./AutoLink.html> and other rarely-used link syntaxes
            diag.span_suggestion_verbose(
                span,
                "consider linking by path instead",
                format!("[{intra_doc_link}][]"),
                Applicability::MachineApplicable,
            );
        }
    });
}

/// Parse relative links,
/// such as `../other_module/struct.Foo.html`
fn check_relative(mut current_item: LocalDefId, mut dest_url: &str, cx: &LateContext<'_>) -> Option<String> {
    // The directory always corresponds to a module.
    while !matches!(cx.tcx.def_kind(current_item.to_def_id()), DefKind::Mod) {
        current_item = cx.tcx.local_parent(current_item);
    }
    let module_path = cx.tcx.def_path(current_item.to_def_id());
    let mut module_path_data = module_path
        .data
        .iter()
        .map(|data| data.data)
        .collect::<Vec<DefPathData>>();
    assert_ne!(module_path_data.first(), Some(&DefPathData::CrateRoot));
    module_path_data.insert(0, DefPathData::CrateRoot);

    // count how many times `../` was written at the start of the relative path
    let mut path_component_parent_count = 0;
    while dest_url.starts_with('.') {
        if dest_url.starts_with("../") {
            dest_url = &dest_url[3..];
            if path_component_parent_count > module_path_data.len() {
                // if the `../` reaches module_path_data.len, then that means
                // we've gone above the crate root, and are looking at
                // another crate
                //
                // if it's module_path_data.len + 1, then we're above the doc
                // bundle entirely
                return None;
            }
            path_component_parent_count += 1;
        } else if dest_url.starts_with("./") {
            dest_url = &dest_url[2..];
        } else {
            // path component starts with `.`, but not with `../` or `./`
            // module names can't contain `.` in Rust, so this can't be
            // a link to an item in a module
            return None;
        }
    }
    let mut parent_path_components =
        symbols_for_def_path_data(&module_path_data[..module_path_data.len() - path_component_parent_count]);

    if (dest_url == "index.html" || dest_url.is_empty()) && !parent_path_components.is_empty() {
        let parent_path = parent_path_components
            .iter()
            .map(Symbol::as_str)
            .collect::<Vec<&str>>()
            .join("::");
        return Some(format!("mod@{parent_path}"));
    }

    // parse the URL after the `../`s
    let (mut path_components, disambiguator, item_name) = check_url_inner(dest_url)?;

    // find the path to the crate, if any
    let crate_path_components = if parent_path_components.is_empty()
        && let crate_name = path_components.first().copied().unwrap_or(item_name)
        && let Some(crate_path_components) = cx.tcx.crates(()).iter().find_map(|&cnum| {
            let extern_crate = cx.tcx.extern_crate(cnum)?;
            // manual intra-doc links are only redundant if the crate is already a dependency
            if !extern_crate.is_direct() || cx.tcx.crate_name(cnum).as_str() != crate_name {
                return None;
            }
            match extern_crate.src {
                ExternCrateSource::Path => Some(vec![cx.tcx.crate_name(cnum)]),
                ExternCrateSource::Extern(did) => Some(symbols_for_def_path_data(
                    &std::iter::once(DefPathData::CrateRoot)
                        .chain(cx.tcx.def_path(did).data.iter().map(|data| data.data))
                        .collect::<Vec<_>>(),
                )),
            }
        }) {
        if path_components.is_empty() {
            // This is a link directly to the crate.
            return Some(format!(
                "mod@{}",
                crate_path_components
                    .iter()
                    .map(Symbol::as_str)
                    .collect::<Vec<&str>>()
                    .join("::")
            ));
        }
        path_components.remove(0);
        crate_path_components
    } else if !parent_path_components.is_empty() {
        vec![parent_path_components.remove(0)]
    } else {
        // path always contains at least a crate
        return None;
    };
    let crate_path = crate_path_components
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<&str>>()
        .join("::");

    let parent_path = parent_path_components
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<&str>>()
        .join("::");
    let path = path_components.join("::");
    Some(format!(
        "{disambiguator}@{crate_path}{sep1}{parent_path}{sep2}{path}::{item_name}",
        sep1 = if crate_path.is_empty() || parent_path.is_empty() {
            ""
        } else {
            "::"
        },
        sep2 = if path.is_empty() || (parent_path.is_empty() && crate_path.is_empty()) {
            ""
        } else {
            "::"
        },
    ))
}

/// Parse docs.rs links,
/// if the link points at a crate that is also a dependency.
fn check_docsrs(mut dest_url: &str, cx: &LateContext<'_>) -> Option<String> {
    if dest_url.starts_with("https://docs.rs/") {
        dest_url = &dest_url[16..];
    } else {
        // not a docs.rs link
        return None;
    }
    let (_package_name, version_and_path) = dest_url.split_once('/')?;
    let Some(("latest", path)) = version_and_path.split_once('/') else {
        // if a version is specified, it might be different from the
        // actual dependency version
        return None;
    };
    let (crate_name, path) = path.split_once('/')?;
    let crate_path = cx.tcx.crates(()).iter().find_map(|&cnum| {
        let extern_crate = cx.tcx.extern_crate(cnum)?;
        // docs.rs links are only redundant if the crate is already a dependency
        if !extern_crate.is_direct() || cx.tcx.crate_name(cnum).as_str() != crate_name {
            return None;
        }
        match extern_crate.src {
            ExternCrateSource::Path => Some(vec![cx.tcx.crate_name(cnum)]),
            ExternCrateSource::Extern(did) => Some(symbols_for_def_path_data(
                &std::iter::once(DefPathData::CrateRoot)
                    .chain(cx.tcx.def_path(did).data.iter().map(|data| data.data))
                    .collect::<Vec<_>>(),
            )),
        }
    })?;

    let crate_path = crate_path.iter().map(Symbol::as_str).collect::<Vec<&str>>().join("::");

    if path == "index.html" || path.is_empty() {
        return Some(format!("mod@{crate_path}"));
    }

    let (path_components, disambiguator, item_name) = check_url_inner(path)?;

    let path = path_components.join("::");
    Some(format!(
        "{disambiguator}@{crate_path}{sep}{path}::{item_name}",
        sep = if crate_path.is_empty() || path.is_empty() {
            ""
        } else {
            "::"
        }
    ))
}

fn check_url_inner(mut path: &str) -> Option<(Vec<&str>, &str, &str)> {
    let mut path_components = Vec::with_capacity(path.bytes().filter(|c| *c == b'/').count() + 1);
    while let Some((dirname, child_path)) = path.split_once('/') {
        if dirname.contains('.') || dirname.is_empty() {
            // module names can't contain `.` in Rust, so this can't be
            // a link to an item in a module
            return None;
        }
        path_components.push(dirname);
        path = child_path;
    }

    if path == "index.html" || path.is_empty() {
        let item_name = path_components.pop()?;
        return Some((path_components, "mod", item_name));
    }

    let filename_parts = path.split('.').collect::<Vec<_>>();
    let &[disambiguator, item_name, html] = &filename_parts[..] else {
        // filename doesn't file pattern `DISAMBIGUATOR.MyName.html`
        return None;
    };
    if html != "html"
        || !matches!(
            disambiguator,
            // copied from librustdoc/formats/item_type.rs
            |"mod"| "externcrate"
                | "import"
                | "struct"
                | "union"
                | "enum"
                | "fn"
                | "type"
                | "static"
                | "trait"
                | "impl"
                | "tymethod"
                | "method"
                | "structfield"
                | "variant"
                | "macro"
                | "primitive"
                | "associatedtype"
                | "constant"
                | "associatedconstant"
                | "foreigntype"
                | "keyword"
                | "attr"
                | "derive"
                | "traitalias"
                | "attribute"
        )
    {
        // filename doesn't file pattern `DISAMBIGUATOR.MyName.html`
        return None;
    }

    Some((path_components, disambiguator, item_name))
}

/// Generate an intra-doc link for a defpath
/// that should never contain non-module parents.
fn symbols_for_def_path_data(data: &[DefPathData]) -> Vec<Symbol> {
    data.iter()
        .map(|part| {
            if *part == DefPathData::CrateRoot {
                kw::Crate
            } else {
                part.get_opt_name()
                    .expect("a module's parents are the crate, plus other modules")
            }
        })
        .collect()
}
