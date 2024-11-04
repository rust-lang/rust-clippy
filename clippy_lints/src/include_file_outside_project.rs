use rustc_ast::{Attribute, LitKind, MetaItem, MetaItemInner};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{Expr, ExprKind, HirId, Item};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::{FileName, Span, sym};

use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::root_macro_call_first_node;

use cargo_metadata::MetadataCommand;

use std::path::{Path, PathBuf};

declare_clippy_lint! {
    /// ### What it does
    /// Check if files included with one of the `include` macros (ie, `include!`, `include_bytes!`
    /// and `include_str!`) or the `path` attribute are actually part of the project.
    ///
    /// ### Why is this bad?
    /// If the included file is outside of the project folder, it will not be part of the releases,
    /// prevent project to work when others use it.
    ///
    /// ### Example
    /// ```ignore
    /// let x = include_str!("/etc/passwd");
    ///
    /// #[path = "/etc/passwd"]
    /// mod bar;
    /// ```
    /// Use instead:
    /// ```ignore
    /// let x = include_str!("./passwd");
    ///
    /// #[path = "./passwd"]
    /// mod bar;
    /// ```
    #[clippy::version = "1.84.0"]
    pub INCLUDE_FILE_OUTSIDE_PROJECT,
    suspicious,
    "checks that all included files are inside the project folder"
}

pub(crate) struct IncludeFileOutsideProject {
    cargo_manifest_dir: Option<PathBuf>,
    warned_spans: FxHashSet<PathBuf>,
    can_check_crate: bool,
}

impl_lint_pass!(IncludeFileOutsideProject => [INCLUDE_FILE_OUTSIDE_PROJECT]);

impl IncludeFileOutsideProject {
    pub(crate) fn new(conf: &'static Conf) -> Self {
        let mut can_check_crate = true;
        if !conf.cargo_ignore_publish {
            match MetadataCommand::new().no_deps().exec() {
                Ok(metadata) => {
                    for package in &metadata.packages {
                        // only run the lint if publish is `None` (`publish = true` or skipped entirely)
                        // or if the vector isn't empty (`publish = ["something"]`)
                        if !matches!(package.publish.as_deref(), Some([]) | None) {
                            can_check_crate = false;
                            break;
                        }
                    }
                },
                Err(_) => can_check_crate = false,
            }
        }

        Self {
            cargo_manifest_dir: std::env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from),
            warned_spans: FxHashSet::default(),
            can_check_crate,
        }
    }

    fn check_file_path(&mut self, cx: &LateContext<'_>, span: Span) {
        if span.is_dummy() {
            return;
        }
        let source_map = cx.tcx.sess.source_map();
        let file = source_map.lookup_char_pos(span.lo()).file;
        if let FileName::Real(real_filename) = file.name.clone()
            && let Some(file_path) = real_filename.into_local_path()
            && let Ok(file_path) = file_path.canonicalize()
            // Only lint once per path for `include` macros.
            && !self.warned_spans.contains(&file_path)
            && !self.is_part_of_project_dir(&file_path)
        {
            let span = span.source_callsite();
            self.emit_error(cx, span.with_hi(span.lo()), file_path);
        }
    }

    fn is_part_of_project_dir(&self, file_path: &Path) -> bool {
        if let Some(ref cargo_manifest_dir) = self.cargo_manifest_dir {
            // Check if both paths start with the same thing.
            let mut file_iter = file_path.iter();

            for cargo_item in cargo_manifest_dir {
                match file_iter.next() {
                    Some(file_path) if file_path == cargo_item => {},
                    _ => {
                        // If we enter this arm, it means that the included file path is not
                        // into the cargo manifest folder.
                        return false;
                    },
                }
            }
        }
        true
    }

    fn emit_error(&mut self, cx: &LateContext<'_>, span: Span, file_path: PathBuf) {
        #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
        span_lint_and_then(
            cx,
            INCLUDE_FILE_OUTSIDE_PROJECT,
            span,
            "attempted to include a file outside of the project",
            |diag| {
                diag.note(format!(
                    "file is located at `{}` which is outside of project folder (`{}`)",
                    file_path.display(),
                    self.cargo_manifest_dir.as_ref().unwrap().display(),
                ));
            },
        );
        self.warned_spans.insert(file_path);
    }

    fn check_hir_id(&mut self, cx: &LateContext<'_>, span: Span, hir_id: HirId) {
        if self.cargo_manifest_dir.is_some()
            && let hir = cx.tcx.hir()
            && let Some(parent_hir_id) = hir.parent_id_iter(hir_id).next()
            && let parent_span = hir.span(parent_hir_id)
            && !parent_span.contains(span)
        {
            self.check_file_path(cx, span);
        }
    }

    fn check_attribute(&mut self, cx: &LateContext<'_>, attr: &MetaItem) {
        let Some(ident) = attr.ident() else { return };
        if ident.name == sym::path {
            if let Some(value) = attr.value_str()
                && let Some(span) = attr.name_value_literal_span()
                && let file_path = Path::new(value.as_str())
                && let Ok(file_path) = file_path.canonicalize()
                && !self.is_part_of_project_dir(&file_path)
            {
                self.emit_error(cx, span, file_path);
            }
        } else if ident.name == sym::cfg_attr
            && let Some(&[_, MetaItemInner::MetaItem(ref attr)]) = attr.meta_item_list()
        {
            self.check_attribute(cx, attr);
        }
    }
}

impl LateLintPass<'_> for IncludeFileOutsideProject {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        if !self.can_check_crate {
            return;
        }
        if !expr.span.from_expansion() {
            self.check_hir_id(cx, expr.span, expr.hir_id);
        } else if let ExprKind::Lit(lit) = &expr.kind
            && matches!(lit.node, LitKind::ByteStr(..) | LitKind::Str(..))
            && let Some(macro_call) = root_macro_call_first_node(cx, expr)
            && (cx.tcx.is_diagnostic_item(sym::include_bytes_macro, macro_call.def_id)
                || cx.tcx.is_diagnostic_item(sym::include_str_macro, macro_call.def_id))
        {
            self.check_hir_id(cx, expr.span, expr.hir_id);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'_>, item: &'_ Item<'_>) {
        // Interestingly enough, `include!` content is not considered expanded. Which allows us
        // to easily filter out items we're not interested into.
        if self.can_check_crate && !item.span.from_expansion() {
            self.check_hir_id(cx, item.span, item.hir_id());
        }
    }

    fn check_attributes(&mut self, cx: &LateContext<'_>, attrs: &[Attribute]) {
        if !self.can_check_crate {
            return;
        }
        for attr in attrs {
            if let Some(attr) = attr.meta() {
                self.check_attribute(cx, &attr);
            }
        }
    }
}
