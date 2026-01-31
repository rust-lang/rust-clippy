use clippy_utils::ast_utils::{eq_id, is_useless_with_eq_exprs};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use core::ops::{Add, AddAssign};
use rustc_ast::ast::BinOpKind;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{Expr, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{AssocKind, Ty, TyCtxt};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::source_map::Spanned;
use rustc_span::symbol::Ident;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unlikely usages of binary operators that are almost
    /// certainly typos and/or copy/paste errors, given the other usages
    /// of binary operators nearby.
    ///
    /// ### Why is this bad?
    /// They are probably bugs and if they aren't then they look like bugs
    /// and you should add a comment explaining why you are doing such an
    /// odd set of operations.
    ///
    /// ### Known problems
    /// There may be some false positives if you are trying to do something
    /// unusual that happens to look like a typo.
    ///
    /// ### Example
    /// ```no_run
    /// struct Vec3 {
    ///     x: f64,
    ///     y: f64,
    ///     z: f64,
    /// }
    ///
    /// impl Eq for Vec3 {}
    ///
    /// impl PartialEq for Vec3 {
    ///     fn eq(&self, other: &Self) -> bool {
    ///         // This should trigger the lint because `self.x` is compared to `other.y`
    ///         self.x == other.y && self.y == other.y && self.z == other.z
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # struct Vec3 {
    /// #     x: f64,
    /// #     y: f64,
    /// #     z: f64,
    /// # }
    /// // same as above except:
    /// impl PartialEq for Vec3 {
    ///     fn eq(&self, other: &Self) -> bool {
    ///         // Note we now compare other.x to self.x
    ///         self.x == other.x && self.y == other.y && self.z == other.z
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.50.0"]
    pub SUSPICIOUS_OPERATION_GROUPINGS,
    nursery,
    "groupings of binary operations that look suspiciously like typos"
}

declare_lint_pass!(SuspiciousOperationGroupings => [SUSPICIOUS_OPERATION_GROUPINGS]);

impl LateLintPass<'_> for SuspiciousOperationGroupings {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let Some(binops) = extract_related_binops(&expr.kind) {
            check_binops(cx, &binops.iter().collect::<Vec<_>>());

            let mut op_types = Vec::with_capacity(binops.len());
            // We could use a hashmap, etc. to avoid being O(n*m) here, but
            // we want the lints to be emitted in a consistent order. Besides,
            // m, (the number of distinct `BinOpKind`s in `binops`)
            // will often be small, and does have an upper limit.
            binops.iter().map(|b| b.op).for_each(|op| {
                if !op_types.contains(&op) {
                    op_types.push(op);
                }
            });

            for op_type in op_types {
                let ops: Vec<_> = binops.iter().filter(|b| b.op == op_type).collect();

                check_binops(cx, &ops);
            }
        }
    }
}

fn check_binops(cx: &LateContext<'_>, binops: &[&BinaryOp<'_>]) {
    let binop_count = binops.len();
    if binop_count < 2 {
        // Single binary operation expressions would likely be false
        // positives.
        return;
    }

    let mut one_ident_difference_count = 0;
    let mut no_difference_info = None;
    let mut double_difference_info = None;
    let mut expected_ident_loc = None;
    let mut paired_identifiers = FxHashSet::default();

    for (i, BinaryOp { left, right, op, .. }) in binops.iter().enumerate() {
        match ident_difference_expr(left, right) {
            IdentDifference::NoDifference => {
                if is_useless_with_eq_exprs(*op) {
                    // The `eq_op` lint should catch this in this case.
                    return;
                }

                no_difference_info = Some(i);
            },
            IdentDifference::Single(ident_loc) => {
                one_ident_difference_count += 1;
                if let Some(previous_expected) = expected_ident_loc {
                    if previous_expected != ident_loc {
                        // This expression doesn't match the form we're
                        // looking for.
                        return;
                    }
                } else {
                    expected_ident_loc = Some(ident_loc);
                }

                // If there was only a single difference, all other idents
                // must have been the same, and thus were paired.
                for id in skip_index(HirExprIdents::new(left), ident_loc.index) {
                    paired_identifiers.insert(id);
                }
            },
            IdentDifference::Double(ident_loc1, ident_loc2) => {
                double_difference_info = Some((i, ident_loc1, ident_loc2));
            },
            IdentDifference::Multiple | IdentDifference::NonIdent => {
                // It's too hard to know whether this is a bug or not.
                return;
            },
        }
    }

    let mut applicability = Applicability::MachineApplicable;

    if let Some(expected_loc) = expected_ident_loc {
        match (no_difference_info, double_difference_info) {
            (Some(i), None) => attempt_to_emit_no_difference_lint(cx, binops, i, expected_loc),
            (None, Some((double_difference_index, ident_loc1, ident_loc2))) => {
                if one_ident_difference_count == binop_count - 1
                    && let Some(binop) = binops.get(double_difference_index)
                {
                    let changed_loc = if ident_loc1 == expected_loc {
                        ident_loc2
                    } else if ident_loc2 == expected_loc {
                        ident_loc1
                    } else {
                        // This expression doesn't match the form we're
                        // looking for.
                        return;
                    };

                    if let Some(sugg) = ident_swap_sugg(cx, &paired_identifiers, binop, changed_loc, &mut applicability)
                    {
                        emit_suggestion(cx, binop.span, sugg, applicability);
                    }
                }
            },
            _ => {},
        }
    }
}

fn attempt_to_emit_no_difference_lint(
    cx: &LateContext<'_>,
    binops: &[&BinaryOp<'_>],
    i: usize,
    expected_loc: IdentLocation,
) {
    if let Some(binop) = binops.get(i).copied() {
        // We need to try and figure out which identifier we should
        // suggest using instead. Since there could be multiple
        // replacement candidates in a given expression, and we're
        // just taking the first one, we may get some bad lint
        // messages.
        let mut applicability = Applicability::MaybeIncorrect;

        // We assume that the correct ident is one used elsewhere in
        // the other binops, in a place that there was a single
        // difference between idents before.
        let old_left_ident = get_ident(binop.left, expected_loc);
        let old_right_ident = get_ident(binop.right, expected_loc);

        for b in skip_index(binops.iter(), i) {
            if let (Some(old_ident), Some(new_ident)) = (old_left_ident, get_ident(b.left, expected_loc))
                && old_ident != new_ident
                && let Some(sugg) = suggestion_with_swapped_ident(
                    cx,
                    CheckFields::No,
                    binop.left,
                    expected_loc,
                    new_ident,
                    &mut applicability,
                )
            {
                emit_suggestion(
                    cx,
                    binop.span,
                    replace_left_sugg(cx, binop, &sugg, &mut applicability),
                    applicability,
                );
                return;
            }

            if let (Some(old_ident), Some(new_ident)) = (old_right_ident, get_ident(b.right, expected_loc))
                && old_ident != new_ident
                // We check for the receiver of the expression in this case
                // so don't check the fields in order to avoid false negatives
                && let Some(sugg) = suggestion_with_swapped_ident(
                    cx,
                    CheckFields::No,
                    binop.right,
                    expected_loc,
                    new_ident,
                    &mut applicability,
                )
            {
                emit_suggestion(
                    cx,
                    binop.span,
                    replace_right_sugg(cx, binop, &sugg, &mut applicability),
                    applicability,
                );
                return;
            }
        }
    }
}

fn emit_suggestion(cx: &LateContext<'_>, span: Span, sugg: String, applicability: Applicability) {
    span_lint_and_sugg(
        cx,
        SUSPICIOUS_OPERATION_GROUPINGS,
        span,
        "this sequence of operators looks suspiciously like a bug",
        "did you mean",
        sugg,
        applicability,
    );
}

fn ident_swap_sugg(
    cx: &LateContext<'_>,
    paired_identifiers: &FxHashSet<Ident>,
    binop: &BinaryOp<'_>,
    location: IdentLocation,
    applicability: &mut Applicability,
) -> Option<String> {
    let left_ident = get_ident(binop.left, location)?;
    let right_ident = get_ident(binop.right, location)?;

    let sugg = match (
        paired_identifiers.contains(&left_ident),
        paired_identifiers.contains(&right_ident),
    ) {
        (true, true) | (false, false) => {
            // We don't have a good guess of what ident should be
            // used instead, in these cases.
            *applicability = Applicability::MaybeIncorrect;

            // We arbitrarily choose one side to suggest changing,
            // since we don't have a better guess. If the user
            // ends up duplicating a clause, the `logic_bug` lint
            // should catch it.

            let right_suggestion =
                suggestion_with_swapped_ident(cx, CheckFields::Yes, binop.right, location, left_ident, applicability)?;

            replace_right_sugg(cx, binop, &right_suggestion, applicability)
        },
        (false, true) => {
            // We haven't seen a pair involving the left one, so
            // it's probably what is wanted.

            let right_suggestion =
                suggestion_with_swapped_ident(cx, CheckFields::Yes, binop.right, location, left_ident, applicability)?;

            replace_right_sugg(cx, binop, &right_suggestion, applicability)
        },
        (true, false) => {
            // We haven't seen a pair involving the right one, so
            // it's probably what is wanted.
            let left_suggestion =
                suggestion_with_swapped_ident(cx, CheckFields::No, binop.left, location, right_ident, applicability)?;

            replace_left_sugg(cx, binop, &left_suggestion, applicability)
        },
    };

    Some(sugg)
}

fn replace_left_sugg(
    cx: &LateContext<'_>,
    binop: &BinaryOp<'_>,
    left_suggestion: &str,
    applicability: &mut Applicability,
) -> String {
    format!(
        "{left_suggestion} {} {}",
        binop.op.as_str(),
        snippet_with_applicability(cx, binop.right.span, "..", applicability),
    )
}

fn replace_right_sugg(
    cx: &LateContext<'_>,
    binop: &BinaryOp<'_>,
    right_suggestion: &str,
    applicability: &mut Applicability,
) -> String {
    format!(
        "{} {} {right_suggestion}",
        snippet_with_applicability(cx, binop.left.span, "..", applicability),
        binop.op.as_str(),
    )
}

#[derive(Clone, Debug)]
struct BinaryOp<'exprs> {
    op: BinOpKind,
    span: Span,
    left: &'exprs Expr<'exprs>,
    right: &'exprs Expr<'exprs>,
}

impl<'exprs> BinaryOp<'exprs> {
    fn new(op: BinOpKind, (left, right): (&'exprs Expr<'exprs>, &'exprs Expr<'exprs>)) -> Self {
        let span = left.span.to(right.span);

        Self { op, span, left, right }
    }
}

fn strip_non_ident_wrappers<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    let mut output = expr;
    loop {
        output = match &output.kind {
            ExprKind::Unary(_, inner) => inner,
            _ => {
                return output;
            },
        };
    }
}

fn extract_related_binops<'hir>(kind: &'hir ExprKind<'hir>) -> Option<Vec<BinaryOp<'hir>>> {
    append_opt_vecs(chained_binops(kind), if_statement_binops(kind))
}

fn if_statement_binops<'hir>(kind: &'hir ExprKind<'hir>) -> Option<Vec<BinaryOp<'hir>>> {
    match kind {
        ExprKind::If(condition, _, _) => chained_binops(&condition.kind),
        ExprKind::Block(block, _) => {
            let mut output = None;
            for stmt in block.stmts {
                match &stmt.kind {
                    StmtKind::Expr(e) | StmtKind::Semi(e) => {
                        output = append_opt_vecs(output, if_statement_binops(&e.kind));
                    },
                    _ => {},
                }
            }
            output
        },
        _ => None,
    }
}

fn append_opt_vecs<A>(target_opt: Option<Vec<A>>, source_opt: Option<Vec<A>>) -> Option<Vec<A>> {
    match (target_opt, source_opt) {
        (Some(mut target), Some(source)) => {
            target.reserve(source.len());
            for op in source {
                target.push(op);
            }
            Some(target)
        },
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}

fn chained_binops<'hir>(kind: &'hir ExprKind<'hir>) -> Option<Vec<BinaryOp<'hir>>> {
    match kind {
        ExprKind::Binary(_, left_outer, right_outer) => chained_binops_helper(left_outer, right_outer),
        ExprKind::Unary(_, e) => chained_binops(&e.kind),
        _ => None,
    }
}

fn chained_binops_helper<'expr>(
    left_outer: &'expr Expr<'expr>,
    right_outer: &'expr Expr<'expr>,
) -> Option<Vec<BinaryOp<'expr>>> {
    match (&left_outer.kind, &right_outer.kind) {
        (ExprKind::Unary(_, left_e), ExprKind::Unary(_, right_e)) => chained_binops_helper(left_e, right_e),
        (ExprKind::Unary(_, left_e), _) => chained_binops_helper(left_e, right_outer),
        (_, ExprKind::Unary(_, right_e)) => chained_binops_helper(left_outer, right_e),
        (
            ExprKind::Binary(Spanned { node: left_op, .. }, left_left, left_right),
            ExprKind::Binary(Spanned { node: right_op, .. }, right_left, right_right),
        ) => match (
            chained_binops_helper(left_left, left_right),
            chained_binops_helper(right_left, right_right),
        ) {
            (Some(mut left_ops), Some(right_ops)) => {
                left_ops.reserve(right_ops.len());
                for op in right_ops {
                    left_ops.push(op);
                }
                Some(left_ops)
            },
            (Some(mut left_ops), _) => {
                left_ops.push(BinaryOp::new(*right_op, (right_left, right_right)));
                Some(left_ops)
            },
            (_, Some(mut right_ops)) => {
                right_ops.insert(0, BinaryOp::new(*left_op, (left_left, left_right)));
                Some(right_ops)
            },
            (None, None) => Some(vec![
                BinaryOp::new(*left_op, (left_left, left_right)),
                BinaryOp::new(*right_op, (right_left, right_right)),
            ]),
        },
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
struct IdentLocation {
    index: usize,
}

impl Add for IdentLocation {
    type Output = IdentLocation;

    fn add(self, other: Self) -> Self::Output {
        Self {
            index: self.index + other.index,
        }
    }
}

impl AddAssign for IdentLocation {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[derive(Clone, Copy, Debug)]
enum IdentDifference {
    NoDifference,
    Single(IdentLocation),
    Double(IdentLocation, IdentLocation),
    Multiple,
    NonIdent,
}

impl Add for IdentDifference {
    type Output = IdentDifference;

    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Self::NoDifference, output) | (output, Self::NoDifference) => output,
            (Self::Multiple, _)
            | (_, Self::Multiple)
            | (Self::Double(_, _), Self::Single(_))
            | (Self::Single(_) | Self::Double(_, _), Self::Double(_, _)) => Self::Multiple,
            (Self::NonIdent, _) | (_, Self::NonIdent) => Self::NonIdent,
            (Self::Single(il1), Self::Single(il2)) => Self::Double(il1, il2),
        }
    }
}

impl AddAssign for IdentDifference {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl IdentDifference {
    /// Returns true if learning about more differences will not change the value
    /// of this `IdentDifference`, and false otherwise.
    fn is_complete(&self) -> bool {
        match self {
            Self::NoDifference | Self::Single(_) | Self::Double(_, _) => false,
            Self::Multiple | Self::NonIdent => true,
        }
    }
}

fn ident_difference_expr(left: &Expr<'_>, right: &Expr<'_>) -> IdentDifference {
    ident_difference_expr_with_base_location(left, right, IdentLocation::default()).0
}

fn ident_difference_expr_with_base_location(
    left: &Expr<'_>,
    right: &Expr<'_>,
    mut base: IdentLocation,
) -> (IdentDifference, IdentLocation) {
    // Ideally, this function should not use IdentIter because it should return
    // early if the expressions have any non-ident differences. We want that early
    // return because if without that restriction the lint would lead to false
    // positives.
    //
    // But, we cannot (easily?) use a `rustc_hir::intravisit::Visitor`, since we need
    // the two expressions to be walked in lockstep. And without a `Visitor`, we'd
    // have to do all the HIR traversal ourselves, which is a lot of work, since to
    // do it properly we'd need to be able to handle more or less every possible
    // HIR node since `Item`s can be written inside `Expr`s.
    //
    // In practice, it seems likely that expressions, above a certain size, that
    // happen to use the exact same idents in the exact same order, and which are
    // not structured the same, would be rare. Therefore it seems likely that if
    // we do only the first layer of matching ourselves and eventually fallback on
    // IdentIter, then the output of this function will be almost always be correct
    // in practice.
    //
    // If it turns out that problematic cases are more prevalent than we assume,
    // then we should be able to change this function to do the correct traversal,
    // without needing to change the rest of the code.

    #![allow(clippy::enum_glob_use)]
    use ExprKind::*;

    match (
        &strip_non_ident_wrappers(left).kind,
        &strip_non_ident_wrappers(right).kind,
    ) {
        (Yield(_, _), Yield(_, _))
        | (Repeat(_, _), Repeat(_, _))
        | (Struct(_, _, _), Struct(_, _, _))
        | (InlineAsm(_), InlineAsm(_))
        | (Ret(_), Ret(_))
        | (Continue(_), Continue(_))
        | (Break(_, _), Break(_, _))
        | (AddrOf(_, _, _), AddrOf(_, _, _))
        | (Path(_), Path(_))
        | (Index(_, _, _), Index(_, _, _))
        | (Field(_, _), Field(_, _))
        | (AssignOp(_, _, _), AssignOp(_, _, _))
        | (Assign(_, _, _), Assign(_, _, _))
        | (Block(_, _), Block(_, _))
        | (Closure(_), Closure(_))
        | (Match(_, _, _), Match(_, _, _))
        | (Loop(_, _, _, _), Loop(_, _, _, _))
        | (If(_, _, _), If(_, _, _))
        | (Let(_), Let(_))
        | (Type(_, _), Type(_, _))
        | (Cast(_, _), Cast(_, _))
        | (Lit(_), Lit(_))
        | (Unary(_, _), Unary(_, _))
        | (Binary(_, _, _), Binary(_, _, _))
        | (Tup(_), Tup(_))
        | (MethodCall(_, _, _, _), MethodCall(_, _, _, _))
        | (Call(_, _), Call(_, _))
        | (ConstBlock(_), ConstBlock(_))
        | (Array(_), Array(_)) => {
            // keep going
        },
        _ => {
            return (IdentDifference::NonIdent, base);
        },
    }

    let mut difference = IdentDifference::NoDifference;

    let left_iter = HirExprIdents::new(left);
    let right_iter = HirExprIdents::new(right);

    let (new_difference, new_base) = ident_difference_via_ident_iter_with_base_location(left_iter, right_iter, base);

    base = new_base;
    difference += new_difference;

    (difference, base)
}

fn ident_difference_via_ident_iter_with_base_location<I>(
    left: I,
    right: I,
    mut base: IdentLocation,
) -> (IdentDifference, IdentLocation)
where
    I: IntoIterator<Item = Ident>,
{
    // See the note in `ident_difference_expr_with_base_location` about `IdentIter`
    let mut difference = IdentDifference::NoDifference;

    let mut left_iter = left.into_iter();
    let mut right_iter = right.into_iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(left_ident), Some(right_ident)) => {
                if !eq_id(left_ident, right_ident) {
                    difference += IdentDifference::Single(base);
                    if difference.is_complete() {
                        return (difference, base);
                    }
                }
            },
            (Some(_), None) | (None, Some(_)) => {
                return (IdentDifference::NonIdent, base);
            },
            (None, None) => {
                return (difference, base);
            },
        }
        base += IdentLocation { index: 1 };
    }
}

/// A version of `IdentIter` but for the HIR.
struct HirExprIdents(std::vec::IntoIter<Ident>);

impl Iterator for HirExprIdents {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl HirExprIdents {
    fn new(expr: &Expr<'_>) -> Self {
        struct Collector {
            inner: Vec<Ident>,
        }

        impl Visitor<'_> for Collector {
            fn visit_ident(&mut self, ident: Ident) {
                self.inner.push(ident);
            }
        }

        let mut collector = Collector { inner: vec![] };

        collector.visit_expr(expr);

        Self(collector.inner.into_iter())
    }
}

fn get_ident(expr: &Expr<'_>, location: IdentLocation) -> Option<Ident> {
    HirExprIdents::new(expr).nth(location.index)
}

#[derive(Debug, Copy, Clone)]
enum CheckFields {
    Yes,
    No,
}

fn has_inherent_method<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, method_name: Ident) -> bool {
    let Some(adt) = ty.ty_adt_def() else { return false };

    tcx.inherent_impls(adt.did())
        .iter()
        .flat_map(|impl_id| tcx.associated_item_def_ids(impl_id))
        .any(|assoc_id| {
            let item = tcx.associated_item(assoc_id);
            let AssocKind::Fn { name, has_self } = item.kind else {
                return false;
            };

            has_self && name == method_name.name
        });

    false
}

#[track_caller]
fn suggestion_with_swapped_ident(
    cx: &LateContext<'_>,
    mode: CheckFields,
    expr: &Expr<'_>,
    location: IdentLocation,
    new_ident: Ident,
    applicability: &mut Applicability,
) -> Option<String> {
    get_ident(expr, location).and_then(|current_ident| {
        if eq_id(current_ident, new_ident) {
            // We never want to suggest a non-change.
            return None;
        }

        let tys = cx.typeck_results();

        // If this is a method call, extract the method called and it's receiver
        if let ExprKind::MethodCall(path_seg, receiver, _, _) = expr.kind
            && let ExprKind::Field(src, _) = receiver.kind
        {
            // If it's a field expression, extract it's source place.

            // Type of the source.
            let ty_of_src = tys.expr_ty(src);

            // If the source is an ADT (only way we can determine methods)
            // I couldn't particularly find a way to get all inherent methods on
            // something like a `u8` but it's probably possible.
            if let Some(adt_def) = ty_of_src.peel_refs().ty_adt_def() {
                // Iterate over fields of the first variant.
                //
                // `enum`s and `union`s are a bit too complicated to handle,
                // so we're only considering operating on `struct`s.
                let iter = adt_def.variants().iter().next()?.fields.iter();

                iter.filter(|f| f.name != current_ident.name)
                    .map(|f| cx.tcx.type_of(f.did).instantiate_identity())
                    .find(|ty| has_inherent_method(cx.tcx, *ty, path_seg.ident))?;
            }
        }

        if let ExprKind::Field(path_expr, _) = expr.kind {
            let ty_of_receiver = tys.expr_ty(path_expr).peel_refs();

            // If it's not an ADT, the "field" access is going to be either
            // completely invalid or it will be accessing some method on the type.
            //
            // These two cases will be caught during typeck, so we don't have to worry.
            if let Some(adt_def) = ty_of_receiver.ty_adt_def()
                && let CheckFields::Yes = mode
                && !adt_def
                    .variants()
                    .into_iter()
                    // Partially redundant as we are going to be processing structs only
                    // which have only one variant.
                    .flat_map(|variant| variant.fields.iter().map(|field| field.name))
                    .any(|ident| ident == new_ident.name)
            {
                // This is an ADT, however
                // if it doesn't have a field
                // that matches the suggestion
                // then we just bail out.
                return None;
            }
        }

        Some(format!(
            "{}{new_ident}{}",
            snippet_with_applicability(cx, expr.span.with_hi(current_ident.span.lo()), "..", applicability),
            snippet_with_applicability(cx, expr.span.with_lo(current_ident.span.hi()), "..", applicability),
        ))
    })
}

fn skip_index<A, Iter>(iter: Iter, index: usize) -> impl Iterator<Item = A>
where
    Iter: Iterator<Item = A>,
{
    iter.enumerate()
        .filter_map(move |(i, a)| if i == index { None } else { Some(a) })
}
