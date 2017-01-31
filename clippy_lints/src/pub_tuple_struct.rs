use rustc::hir;
use rustc::lint::*;
use rustc::ty;
use utils::span_lint;
use rustc::hir::def_id::DefId;
use rustc::ty::subst::Substs;
use std::iter::repeat;

/// **What it does:** Checks for public tuple structs with private fields
///
/// **Why is this bad?** You can pattern match on the number of fields,
/// thus making it a breaking change to add more fields.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// pub Foo(i32, u32);
/// ```
declare_restriction_lint! {
    pub PUB_TUPLE_STRUCT_WITH_PRIVATE_FIELDS,
    "a public tuple struct with private fields"
}

/// **What it does:** Checks for pattern matching on a non-local tuple struct
/// which has private fields.
///
/// **Why is this bad?** Your code will break in case the number of tuple struct
/// fields change
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// let Foo(_, _) = create_foo();
/// ```
declare_lint! {
    pub MATCH_PRIVATE_TUPLE_STRUCT_FIELDS,
    Warn,
    "matching on a tuple struct's private fields"
}

pub struct Pass;

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(PUB_TUPLE_STRUCT_WITH_PRIVATE_FIELDS, MATCH_PRIVATE_TUPLE_STRUCT_FIELDS)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for Pass {
    fn check_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx hir::Item) {
        if item.vis == hir::Visibility::Public {
            if let hir::ItemStruct(hir::VariantData::Tuple(ref fields, _), _) = item.node {
                if fields.iter().any(|field| field.vis != hir::Visibility::Public) {
                    span_lint(cx,
                              PUB_TUPLE_STRUCT_WITH_PRIVATE_FIELDS,
                              item.span,
                              &format!("the number of fields in this tuple struct\
                                        cannot be changed anymore, since users may\
                                        depend on it at compiletime with `let {}({}) = ...;`",
                                       item.name,
                                       repeat("_, ").take(fields.len()).collect::<String>()));
                }
            }
        }
    }
    fn check_pat(&mut self, cx: &LateContext<'a, 'tcx>, pat: &'tcx hir::Pat) {
        if let hir::PatKind::TupleStruct(_, _, None) = pat.node {
            if let ty::TyAdt(def, substs) = cx.tables.pat_ty(pat).sty {
                if !def.did.is_local() && !def.is_enum() &&
                   def.struct_variant().fields.iter().any(|field| field.vis != ty::Visibility::Public) {
                    use rustc::util::ppaux;
                    use std::fmt;
                    struct Instance<'tcx>(DefId, &'tcx Substs<'tcx>);
                    impl<'tcx> ::std::panic::UnwindSafe for Instance<'tcx> {}
                    impl<'tcx> ::std::panic::RefUnwindSafe for Instance<'tcx> {}
                    impl<'tcx> fmt::Display for Instance<'tcx> {
                        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            ppaux::parameterized(f, self.1, self.0, &[])
                        }
                    }
                    span_lint(cx,
                              MATCH_PRIVATE_TUPLE_STRUCT_FIELDS,
                              pat.span,
                              &format!("this pattern match will break if the author of `{}` changes the number of \
                                        private fields",
                                    Instance(def.did, substs)));
                }
            }
        }
    }
}
