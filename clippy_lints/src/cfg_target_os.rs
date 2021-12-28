//! checks for cfg `target_os` usages

use clippy_utils::diagnostics::span_lint_and_then;
use if_chain::if_chain;
use rustc_ast::{Attribute, MacCall, MetaItemKind, NestedMetaItem};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_parse::parser;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::source_map::Span;
use rustc_span::sym;

static UNIX_SYSTEMS: &[&str] = &[
    "android",
    "dragonfly",
    "emscripten",
    "freebsd",
    "fuchsia",
    "haiku",
    "illumos",
    "ios",
    "l4re",
    "linux",
    "macos",
    "netbsd",
    "openbsd",
    "redox",
    "solaris",
    "vxworks",
];

// NOTE: windows is excluded from the list because it's also a valid target family.
static NON_UNIX_SYSTEMS: &[&str] = &["hermit", "none", "wasi"];

declare_clippy_lint! {
    /// ### What it does
    /// Checks for cfg attributes having operating systems used in target family position.
    ///
    /// ### Why is this bad?
    /// The configuration option will not be recognised and the related item will not be included
    /// by the conditional compilation engine.
    ///
    /// ### Example
    /// Bad:
    /// ```rust
    /// #[cfg(linux)]
    /// fn conditional() { }
    /// ```
    ///
    /// Good:
    /// ```rust
    /// #[cfg(target_os = "linux")]
    /// fn conditional() { }
    /// ```
    ///
    /// Or:
    /// ```rust
    /// #[cfg(unix)]
    /// fn conditional() { }
    /// ```
    /// Check the [Rust Reference](https://doc.rust-lang.org/reference/conditional-compilation.html#target_os) for more details.
    #[clippy::version = "1.45.0"]
    pub MISMATCHED_TARGET_OS,
    correctness,
    "usage of `cfg(operating_system)` instead of `cfg(target_os = \"operating_system\")`"
}

declare_lint_pass!(CfgTargetOs => [MISMATCHED_TARGET_OS]);

impl EarlyLintPass for CfgTargetOs {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        check_mismatched_target_os(cx, attr);
    }

    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        check_mismatched_target_os_mac(cx, mac);
    }
}

fn find_os(name: &str) -> Option<&'static str> {
    UNIX_SYSTEMS
        .iter()
        .chain(NON_UNIX_SYSTEMS.iter())
        .find(|&&os| os == name)
        .copied()
}

fn is_unix(name: &str) -> bool {
    UNIX_SYSTEMS.iter().any(|&os| os == name)
}

fn find_mismatched_target_os(items: &[NestedMetaItem]) -> Vec<(&str, Span)> {
    let mut mismatched = Vec::new();

    for item in items {
        if let NestedMetaItem::MetaItem(meta) = item {
            match &meta.kind {
                MetaItemKind::List(list) => {
                    mismatched.extend(find_mismatched_target_os(list));
                },
                MetaItemKind::Word => {
                    if_chain! {
                        if let Some(ident) = meta.ident();
                        if let Some(os) = find_os(&*ident.name.as_str());
                        then {
                            mismatched.push((os, ident.span));
                        }
                    }
                },
                MetaItemKind::NameValue(..) => {},
            }
        }
    }

    mismatched
}

fn write_diagnoses(cx: &EarlyContext<'_>, mismatched: Vec<(&str, Span)>, span: Span) {
    if !mismatched.is_empty() {
        let mess = "operating system used in target family position";

        span_lint_and_then(cx, MISMATCHED_TARGET_OS, span, mess, |diag| {
            // Avoid showing the unix suggestion multiple times in case
            // we have more than one mismatch for unix-like systems
            let mut unix_suggested = false;

            for (os, span) in mismatched {
                let sugg = format!("target_os = \"{}\"", os);
                diag.span_suggestion(span, "try", sugg, Applicability::MaybeIncorrect);

                if !unix_suggested && is_unix(os) {
                    diag.help("did you mean `unix`?");
                    unix_suggested = true;
                }
            }
        });
    }
}

fn check_mismatched_target_os_mac(cx: &EarlyContext<'_>, mac: &MacCall) {
    let args = &mac.args;
    let inner = args.inner_tokens();
    let mut parser = parser::Parser::new(&cx.sess.parse_sess, inner, false, None);
    let meta = parser.parse_meta_item().unwrap();

    let mismatched = if let Some(items) = meta.meta_item_list() {
        find_mismatched_target_os(items)
    } else {
        match &meta.kind {
            MetaItemKind::List(list) => find_mismatched_target_os(list),
            MetaItemKind::Word => {
                if_chain! {
                    if let Some(ident) = meta.ident();
                    if let Some(os) = find_os(&*ident.name.as_str());
                    then {
                        vec![(os, ident.span)]
                    } else {
                        Vec::new()
                    }
                }
            },
            MetaItemKind::NameValue(..) => Vec::new(),
        }
    };

    write_diagnoses(cx, mismatched, mac.span());
}

fn check_mismatched_target_os(cx: &EarlyContext<'_>, attr: &Attribute) {
    if_chain! {
        if attr.has_name(sym::cfg);
        if let Some(list) = attr.meta_item_list();
        let mismatched = find_mismatched_target_os(&list);
        then {
            write_diagnoses(cx, mismatched, attr.span);
        }
    }
}
