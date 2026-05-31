use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::has_repr_attr;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for structs with too many fields.
    ///
    /// ### Why is this bad?
    /// Large structs are often a sign that a type is taking on too many
    /// responsibilities at once.
    ///
    /// As more fields accumulate, it gets harder to work with logically related
    /// subsections of the state. In particular, mutable borrowing often becomes
    /// awkward because independent operations still have to borrow the same large
    /// struct.
    ///
    /// Grouping related fields into smaller helper structs can make the code
    /// easier to read, easier to borrow from, and easier to extend with methods.
    ///
    /// The `too-many-fields-threshold` configuration accepts `0`, which lints on
    /// any non-unit struct.
    ///
    /// ### Example
    /// ```rust,ignore
    /// struct RenderContext {
    ///     theme: Theme,
    ///     font_size: u16,
    ///     font_family: String,
    ///     line_height: u16,
    ///     margin_top: u16,
    ///     margin_right: u16,
    ///     margin_bottom: u16,
    ///     margin_left: u16,
    ///     page_width: u16,
    ///     page_height: u16,
    ///     show_line_numbers: bool,
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// struct Typography {
    ///     font_size: u16,
    ///     font_family: String,
    ///     line_height: u16,
    /// }
    ///
    /// struct Margins {
    ///     top: u16,
    ///     right: u16,
    ///     bottom: u16,
    ///     left: u16,
    /// }
    ///
    /// struct RenderContext {
    ///     theme: Theme,
    ///     typography: Typography,
    ///     margins: Margins,
    ///     page_width: u16,
    ///     page_height: u16,
    ///     show_line_numbers: bool,
    /// }
    /// ```
    #[clippy::version = "1.98.0"]
    pub TOO_MANY_FIELDS,
    pedantic,
    "using too many fields in a struct"
}

impl_lint_pass!(TooManyFields => [TOO_MANY_FIELDS]);

pub struct TooManyFields {
    too_many_fields_threshold: u64,
}

impl TooManyFields {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            too_many_fields_threshold: conf.too_many_fields_threshold,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for TooManyFields {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Struct(_, _, variant_data) = &item.kind
            && variant_data.fields().len() as u64 > self.too_many_fields_threshold
            && !has_repr_attr(cx, item.hir_id())
            && !item.span.from_expansion()
        {
            let fields = variant_data.fields().len();
            let help = if self.too_many_fields_threshold > 0 {
                "consider grouping related fields into a separate struct"
            } else {
                "only fieldless structs are allowed"
            };
            span_lint_and_help(
                cx,
                TOO_MANY_FIELDS,
                item.span,
                format!(
                    "this struct has too many fields ({fields}/{})",
                    self.too_many_fields_threshold
                ),
                None,
                help,
            );
        }
    }
}
