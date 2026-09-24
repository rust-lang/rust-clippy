mod decimal_literal_representation;
mod digit_grouping;
mod mistyped_literal_suffix;

use clippy_config::Conf;
use clippy_utils::numeric_literal::{FloatStr, IntStr, Radix};
use rustc_ast::{ast, token};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Warns if there is a better representation for a numeric literal.
    ///
    /// ### Why restrict this?
    /// Especially for big powers of 2, a hexadecimal representation is usually more
    /// readable than a decimal representation.
    ///
    /// ### Example
    /// ```text
    /// `255` => `0xFF`
    /// `65_535` => `0xFFFF`
    /// `4_042_322_160` => `0xF0F0_F0F0`
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub DECIMAL_LITERAL_REPRESENTATION,
    restriction,
    "using decimal representation when hexadecimal would be better"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns if an integral or floating-point constant is
    /// grouped inconsistently with underscores.
    ///
    /// ### Why is this bad?
    /// Readers may incorrectly interpret inconsistently
    /// grouped digits.
    ///
    /// ### Example
    /// ```no_run
    /// # let _: u64 =
    /// 618_64_9189_73_511
    /// # ;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let _: u64 =
    /// 61_864_918_973_511
    /// # ;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub INCONSISTENT_DIGIT_GROUPING,
    style,
    "integer literals with digits grouped inconsistently"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns if the digits of an integral or floating-point
    /// constant are grouped into groups that
    /// are too large.
    ///
    /// ### Why is this bad?
    /// Negatively impacts readability.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u64 = 6186491_8973511;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub LARGE_DIGIT_GROUPS,
    pedantic,
    "grouping digits into groups that are too large"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns for mistyped suffix in literals
    ///
    /// ### Why is this bad?
    /// This is most probably a typo
    ///
    /// ### Known problems
    /// - Does not match on integers too large to fit in the corresponding unsigned type
    /// - Does not match on `_127` since that is a valid grouping for decimal and octal numbers
    ///
    /// ### Example
    /// ```ignore
    /// `2_32` => `2_i32`
    /// `250_8 => `250_u8`
    /// ```
    #[clippy::version = "1.30.0"]
    pub MISTYPED_LITERAL_SUFFIXES,
    correctness,
    "mistyped literal suffix"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns if a long integral or floating-point constant does
    /// not contain underscores.
    ///
    /// ### Why is this bad?
    /// Reading long numbers is difficult without separators.
    ///
    /// ### Example
    /// ```no_run
    /// # let _: u64 =
    /// 61864918973511
    /// # ;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let _: u64 =
    /// 61_864_918_973_511
    /// # ;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub UNREADABLE_LITERAL,
    pedantic,
    "long literal without underscores"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns if hexadecimal or binary literals are not grouped
    /// by nibble or byte.
    ///
    /// ### Why is this bad?
    /// Negatively impacts readability.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 0xFFF_FFF;
    /// let y: u8 = 0b01_011_101;
    /// ```
    #[clippy::version = "1.49.0"]
    pub UNUSUAL_BYTE_GROUPINGS,
    style,
    "binary or hex literals that aren't grouped by four"
}

impl_lint_pass!(EarlyLiterals => [
    DECIMAL_LITERAL_REPRESENTATION,
    INCONSISTENT_DIGIT_GROUPING,
    LARGE_DIGIT_GROUPS,
    MISTYPED_LITERAL_SUFFIXES,
    UNREADABLE_LITERAL,
    UNUSUAL_BYTE_GROUPINGS,
]);

pub struct EarlyLiterals {
    literal_representation_threshold: u64,
    unreadable_literal_lint_fractions: bool,
}
impl EarlyLiterals {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            literal_representation_threshold: conf.literal_representation_threshold,
            unreadable_literal_lint_fractions: conf.unreadable_literal_lint_fractions,
        }
    }
}

impl EarlyLintPass for EarlyLiterals {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, e: &ast::Expr) {
        if let ast::ExprKind::Lit(ref lit) = e.kind {
            match lit.kind {
                token::LitKind::Integer
                    if let Some(num) = IntStr::from_token_lit(lit)
                        && (num.suffix.is_some()
                            || num.radix == Radix::Bin
                            || !mistyped_literal_suffix::check_int(cx, num, e.span)) =>
                {
                    decimal_literal_representation::check(cx, self.literal_representation_threshold, num, e.span);
                    digit_grouping::check_int(cx, num, e.span);
                },
                token::LitKind::Float
                    if let Some(num) = FloatStr::from_token_lit(lit)
                        && (num.suffix.is_some() || !mistyped_literal_suffix::check_float(cx, &num, e.span)) =>
                {
                    digit_grouping::check_float(cx, &num, self.unreadable_literal_lint_fractions, e.span);
                },
                _ => {},
            }
        }
    }
}
