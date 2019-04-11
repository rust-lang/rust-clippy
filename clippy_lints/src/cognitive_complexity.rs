//! calculate cognitive complexity and warn about overly complex functions

use rustc::lint::{EarlyContext, EarlyLintPass};
use rustc::lint::{LintArray, LintContext, LintPass};
use rustc::{declare_tool_lint, impl_lint_pass};
use syntax::ast::Attribute;
use syntax::ast::*;
use syntax::visit::{walk_expr, Visitor};

use crate::utils::{span_help_and_lint, LimitStack};

declare_clippy_lint! {
    /// **What it does:** Checks for methods with high cognitive complexity.
    ///
    /// **Why is this bad?** Methods of high cognitive complexity tend to be hard to
    /// both read and maintain. Also LLVM will tend to optimize small methods better.
    ///
    /// **Known problems:** Sometimes it's hard to find a way to reduce the
    /// complexity.
    ///
    /// **Example:** Sorry. Examples are too big and varied to put in here. For a
    /// complete explanation of the analysis being made though, you can read this paper:
    /// https://www.sonarsource.com/docs/CognitiveComplexity.pdf
    pub COGNITIVE_COMPLEXITY,
    complexity,
    "functions that should be split up into multiple functions"
}

pub struct CognitiveComplexity {
    limit: LimitStack,
    current_enclosing_function: Option<NodeId>,
}

impl CognitiveComplexity {
    pub fn new(limit: u64) -> Self {
        Self {
            limit: LimitStack::new(limit),
            current_enclosing_function: None,
        }
    }
}

impl_lint_pass!(CognitiveComplexity => [COGNITIVE_COMPLEXITY]);

impl EarlyLintPass for CognitiveComplexity {
    fn check_item_post(&mut self, _: &EarlyContext<'_>, item: &Item) {
        // After processing the inner AST of a function, we unrecord its
        // id, so that other functions can now be recognized and processed.
        if let Some(fn_id) = self.current_enclosing_function {
            if item.id == fn_id {
                self.current_enclosing_function = None;
            }
        }
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Fn(_, _, _, fn_block) = &item.node {
            // Before scoring a function, we check that it's not
            // an inner function. If it was, then we'd be scoring it
            // twice: once for its parent and once for itself.
            if let None = self.current_enclosing_function {
                // Now that we've entered a function, we record it
                // as the current enclosing one. No functions inside it
                // will ever be scored now.
                self.current_enclosing_function = Some(item.id);

                // If the function being explored is marked as "test",
                // then we skip it.
                if item.attrs.iter().any(|a| a.check_name("test")) {
                    return;
                }

                let mut helper = CoCHelper::new();

                helper.visit_block(&fn_block);

                let fn_score = helper.score;
                let score_limit = self.limit.limit();

                if fn_score > score_limit {
                    span_help_and_lint(
                        cx,
                        COGNITIVE_COMPLEXITY,
                        item.span,
                        &format!("the function has a cognitive complexity of {}", fn_score),
                        "you could split it up into multiple smaller functions",
                    );
                }
            }
        }
    }

    fn enter_lint_attrs(&mut self, cx: &EarlyContext<'_>, attrs: &[Attribute]) {
        self.limit.push_attrs(cx.sess(), attrs, "cognitive_complexity");
    }
    fn exit_lint_attrs(&mut self, cx: &EarlyContext<'_>, attrs: &[Attribute]) {
        self.limit.pop_attrs(cx.sess(), attrs, "cognitive_complexity");
    }
}

/// Helps keep track of the Cognitive Complexity Score
/// of a function being analyzed.
struct CoCHelper {
    /// Current Nesting value
    current_nesting: u64,
    /// Current Cognitive Complexity score
    score: u64,
    /// Current Nesting of Binary Logical Operations
    /// (used for proper score calculation)
    logical_binop_nesting: u64,
}

enum ComplexityLevel {
    /// Almost no individual score (Paren, Assign, AssignOp)
    Low,
    /// Most common score (If, Return, Yield)
    Normal,
    // FIXME: delete or populate this case.
    /// High score (no cases yet?)
    High,
    /// Custom score (a catch-all for other cases)
    Custom(u64),
}

impl ComplexityLevel {
    fn get_score(&self) -> u64 {
        match self {
            ComplexityLevel::Low => 2,
            ComplexityLevel::Normal => 10,
            ComplexityLevel::High => 50,
            ComplexityLevel::Custom(score) => *score,
        }
    }
}

impl CoCHelper {
    /// Create a clean CoCHelper
    fn new() -> CoCHelper {
        CoCHelper {
            current_nesting: 0,
            score: 0,
            logical_binop_nesting: 0,
        }
    }

    /// Increment the nesting level by one
    fn push_nesting(&mut self) {
        self.current_nesting += 1;
    }

    /// Decrement the nesting level by one
    fn pop_nesting(&mut self) {
        assert!(self.current_nesting > 0);
        self.current_nesting -= 1;
    }

    /// Mark a determined amount of score
    fn add_to_score(&mut self, amount: u64) {
        self.score += amount;
    }

    /// Mark score for a Nesting-Dependent Structure
    fn score_nd_structure(&mut self, level: ComplexityLevel) {
        self.add_to_score(self.current_nesting * level.get_score());
        self.score_ni_structure(level);
    }

    /// Mark score for a Nesting-Independent Structure
    fn score_ni_structure(&mut self, level: ComplexityLevel) {
        self.add_to_score(level.get_score());
    }

    /// Let the helper know that we've entered a Binary Logical Operation
    fn enter_logical_binop(&mut self) {
        // We score once every time we enter a *new*
        // binary logical operation.
        // That way, the score for chains is
        // `1 + (number of operator changes in the chain)`
        if self.logical_binop_nesting == 0 {
            self.score_ni_structure(ComplexityLevel::Normal);
        }
        self.logical_binop_nesting += 1;
    }

    /// Let the helper know that we've exited a Binary Logical Operation
    fn exit_logical_binop(&mut self) {
        self.logical_binop_nesting -= 1;
    }
}

impl<'ast> Visitor<'ast> for CoCHelper {
    /*
    # Implemented here:

    ## Nesting Structures
        IMPORTANT: ExprKind::Block(..)
        already covers all cases.
    ## (Nesting-Dependent) Increments
        if
        match
        for, while, loop
    ## (Nesting-Independent) Increments
        break, continue
        Sequences of binary logical operators
        Function calls
        Macro calls
    */

    fn visit_expr(&mut self, ex: &'ast Expr) {
        match ex.node {
            // Nesting-Increasing (the one and only)
            ExprKind::Block(..) => {
                self.push_nesting();
                walk_expr(self, ex);
                self.pop_nesting();
            },

            ExprKind::Closure(.., _)
            | ExprKind::IfLet(..)
            | ExprKind::Lit(..)
            | ExprKind::Try(..) => {
                // "If Let" and "Try" are free of own increment.
                // This is because they are language constructs
                // specifically designed to save on complexity.
                walk_expr(self, ex);
            },

            // FIXME (FAR FUTURE): make a separate, documented case, for recursive calls,
            // such that it's treated differently from function or method calls.
            ExprKind::Call(..) | ExprKind::MethodCall(..) => {
                self.score_ni_structure(ComplexityLevel::Normal);
                walk_expr(self, ex);
            },

            // Nesting-Dependent
            ExprKind::If(..)
            | ExprKind::Match(..)
            | ExprKind::ForLoop(..)
            | ExprKind::While(..)
            | ExprKind::Loop(..)
            | ExprKind::WhileLet(..) => {
                // (For the IF-Case)
                // Important: this pays for one "if" and one "else".
                // Every "if" in an "else if" comes here again to pay
                // for itself and its subsequent else.
                self.score_nd_structure(ComplexityLevel::Normal);
                walk_expr(self, ex);
            },

            // Nesting-Independent
            ExprKind::Mac(ref mac) => {
                self.visit_mac(mac);
            },

            // Nesting-Independent
            ExprKind::Continue(_) => {
                self.score_ni_structure(ComplexityLevel::Normal);
            },

            // Nesting-Independent,
            // Sometimes Nesting
            ExprKind::Break(_, ref maybe_inner_ex) => {
                self.score_ni_structure(ComplexityLevel::Normal);
                if let Some(ref inner_ex) = maybe_inner_ex {
                    walk_expr(self, inner_ex);
                }
            },

            // (Nesting-Independent) When boolean operators change, we add 1 to the score.
            ExprKind::Binary(binop, ref l_ex, ref r_ex) => {
                // Here, we're either looking for the leftmost logical operator on the right side,
                // or the rightmost logical operator on the left side. It looks like this:
                //
                // Let's say our Expr is `(a && b) || ((c ^ d) & e)`, and its AST:
                //
                //                          Or
                //                        /    \
                //                       /      \
                //                      /        \
                //                 Paren          Paren
                //                  |               |
                //                 And            BitAnd
                //                /   \          /      \
                //               a     b        /        \
                //                           Paren        e
                //                             |
                //                            Xor
                //                           /   \
                //                          c     d
                //
                // Then, when we call `log_op_at(right_branch, At:LeftMostSide)`,
                // We're looking for that Xor at the leftmost side of the right branch:
                //                          Or
                //                        /    \
                //                       /      \
                //                      /        \
                //                 Paren          Paren
                //                  |               |
                //                 And            BitAnd
                //                /   \          /      \
                //               a     b        /        \
                //                           Paren        e
                //                             |
                //                            Xor <~ THIS ONE :D
                //                           /   \
                //                          c     d
                //
                // Doing this, we can effectively mark a score whenever there is a change
                // in the current chain of logical operators.
                //
                // So say for example, that we're scoring `a && b && c || d && e`.
                // There are 2 changes in the operator chain in this expression,
                // once at `c || d` (it changes from `&&` to `||`) and once at
                // `d && e` (it changes from `||` to `&&`).
                //
                // In order for us to score this change, regardless of the shape of
                // the AST, we need to be able to know which operator sits right
                // next to the current one. If it's then a different operator,
                // we know there is a change in the chain, and we can score it.
                //
                // But what about scoring it twice? Will we see the same change
                // more than once?
                // The answer is no: since ASTs are recursive, child operators
                // can't see their parent operators. Given we're only scoring
                // a change whenever the operators right next to the current one
                // are different to it, AND in our subsequent calls the current
                // operator will not be visible, it's effectively impossible
                // to score this change in the chain more than once.

                /// A location in the AST.
                enum At {
                    LeftMostSide,
                    RightMostSide,
                }

                /// A logical operator
                #[derive(PartialEq)]
                enum LogOp {
                    LogAnd, // &&
                    LogOr,  // ||
                    BitAnd, // &
                    BitOr,  // |
                    BitXor, // ^
                    None,   // Other
                }

                /// Translate from a binary operator to a logical operator
                fn log_op_from_bin_op(bop_kind: BinOpKind) -> LogOp {
                    match bop_kind {
                        BinOpKind::And => LogOp::LogAnd,
                        BinOpKind::Or => LogOp::LogOr,
                        BinOpKind::BitAnd => LogOp::BitAnd,
                        BinOpKind::BitOr => LogOp::BitOr,
                        BinOpKind::BitXor => LogOp::BitXor,
                        _ => LogOp::None,
                    }
                }

                /// Find the rightmost or leftmost logical operator inside of the given `Expr`
                fn log_op_at(expr: &Expr, at: At) -> LogOp {
                    match &expr.node {
                        ExprKind::Binary(binop, ref left_side, ref right_side) => {
                            let current_operator = log_op_from_bin_op(binop.node);

                            let next_operator = match at {
                                At::LeftMostSide => log_op_at(left_side, at),
                                At::RightMostSide => log_op_at(right_side, at),
                            };

                            match next_operator {
                                LogOp::None => current_operator,
                                _ => next_operator,
                            }
                        },
                        ExprKind::Paren(expr) | ExprKind::Unary(_, expr) => log_op_at(&expr, at),
                        _ => LogOp::None,
                    }
                }

                let current_log_op = log_op_from_bin_op(binop.node);

                let is_log_op = current_log_op != LogOp::None;

                if is_log_op {
                    // Here we separate the left and right branches, and go looking
                    // for the rightmost and leftmost logical operator in them, respectively
                    let op_at_left_side = log_op_at(l_ex, At::RightMostSide);
                    let op_at_right_side = log_op_at(r_ex, At::LeftMostSide);

                    if op_at_left_side != LogOp::None && current_log_op != op_at_left_side {
                        self.score_ni_structure(ComplexityLevel::Normal);
                    }

                    if op_at_right_side != LogOp::None && current_log_op != op_at_right_side {
                        self.score_ni_structure(ComplexityLevel::Normal);
                    }

                    self.enter_logical_binop();
                }

                walk_expr(self, ex);

                if is_log_op {
                    self.exit_logical_binop();
                }
            },

            /*
                Low complexity cases
            */

            // (...)
            // ExprKind::Paren(..) => {},

            // # a += bar()
            // ExprKind::AssignOp(..) => {},

            // # a = foo()
            // ExprKind::Assign(..) => {},

            // # foo[2]
            // ExprKind::Index(..) => {},

            // # a.count, or b.0
            // ExprKind::Field(..) => {},

            // # &a or &mut a
            // ExprKind::AddrOf(..) => {},

            // !a, *b
            // ExprKind::Unary(..) => {},

            /*
                Medium complexity cases
            */

            // Return and Yield have the same cog. complexity
            // ExprKind::Ret(..) => {},
            // ExprKind::Yield(..) => {},

            // # foo as f32
            // ExprKind::Cast(..) => {},

            // # Struct literal: Foo { (things) }
            // ExprKind::Struct(..) => {},

            // # (a, b, c)
            // ExprKind::Tup(..) => {},

            // # [a, b, c, d]
            // ExprKind::Array(..) => {},

            // # m..n
            // ExprKind::Range(..) => {},

            /*
                ### Pending ones (FIXME) ###
            */

            // # [m; n]
            // ExprKind::Repeat(..) => {},

            // Haven't used these. Investigate further.
            // ExprKind::TryBlock(..) => {},

            // # Variable reference??
            // ExprKind::Path(..) => {},

            // # (unstable) `box a` syntax
            // ExprKind::Box(..) => {},

            // # FIXME: what is this?
            // ExprKind::ObsoleteInPlace(..) => {},

            // What is Type Ascription??
            // ExprKind::Type(..) => {},

            // Unstable, leave it for after the MVP.
            // ExprKind::Async(..) => {},

            // # asm!(), basically, inline assembly
            // ExprKind::InlineAsm(..) => {},

            // Ill formed expressions.
            ExprKind::Err => {
                panic!("Found an ExprKind::Err. Is this a compiler bug??");
            },

            _ => {
                walk_expr(self, ex);
            },
        }
    }

    fn visit_mac(&mut self, _mac: &Mac) {
        // We override this so that the compiler
        // doesn't panic. See the original implementation
        // of `visit_mac` at rustc's src/libsyntax/visit.rs
        // to know what normally happens.
        self.score_ni_structure(ComplexityLevel::Normal);
    }
}
