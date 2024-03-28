use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet, snippet_opt};
use clippy_utils::ty::expr_type_is_certain;
use clippy_utils::{is_diagnostic_item_or_ctor, is_from_proc_macro, is_lang_item_or_ctor, is_trait_item, std_or_core};
use itertools::{Either, Itertools};
use rustc_ast::{FloatTy, IntTy, Mutability, UintTy};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{ArrayLen, Expr, ExprKind, GenericArgs, LangItem, PrimTy, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, ast_int_ty, ast_uint_ty, Ty};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Span};
use std::fmt::{self, Display, Formatter};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for types constructed by `default` that really don't need to be.
    ///
    /// ### Why is this bad?
    /// It's harder for the reader to know what the value is, and it's an unnecessary function call.
    ///
    /// ### Known problems
    /// Lints `cfg`-dependent type aliases if the type is not explicitly specified (Something like
    /// `Default::default` instead of `<ty>::default`).
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a = A(Option::default());
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let a = A(None);
    /// ```
    #[clippy::version = "1.72.0"]
    pub TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
    pedantic,
    "checks for usage of `Default::default` to construct trivial types"
}
declare_lint_pass!(TrivialDefaultConstructedTypes => [TRIVIAL_DEFAULT_CONSTRUCTED_TYPES]);

impl<'tcx> LateLintPass<'tcx> for TrivialDefaultConstructedTypes {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !expr.span.from_expansion()
            && let ExprKind::Call(call, _) = expr.kind
            && let ExprKind::Path(qpath) = call.kind
            && is_trait_item(cx, call, sym::Default)
        {
            let Some(ty) = (match qpath {
                // `Default::default`
                QPath::Resolved(None, _) => {
                    Type::from_ty(
                        cx,
                        // This is necessary as otherwise we'll get the pre-normalization type,
                        // which in this case is always `Self`. We don't want to check the
                        // `expr`'s type directly as *I think* that will be subject to type
                        // inference.
                        cx.typeck_results().expr_ty(call).fn_sig(cx.tcx).output().skip_binder(),
                        expr_type_is_certain(cx, expr),
                    )
                },
                // `<ty>::default`
                QPath::TypeRelative(ty, _) => Type::from_hir_ty(cx, ty, expr_type_is_certain(cx, expr)),
                _ => return,
            }) else {
                return;
            };

            let msg = match ty.peel_refs() {
                Type::Tuple(tys) => {
                    if tys.is_empty() {
                        "constructing a trivial unit using `Default::default`"
                    } else {
                        "constructing a trivial tuple using `Default::default`"
                    }
                },
                Type::Array(_, _) => "constructing a trivial array using `Default::default`",
                Type::Slice(_) => "constructing a trivial slice using `Default::default`",
                _ => "constructing a trivial type using `Default::default`",
            };

            if !is_from_proc_macro(cx, expr) && !matches!(ty.peel_refs(), Type::Unchanged(_)) {
                span_lint_and_sugg(
                    cx,
                    TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
                    expr.span,
                    msg,
                    "try",
                    ty.display(cx).to_string(),
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

#[derive(Debug)]
enum Type {
    Ref(Mutability, Box<Type>),
    Bool,
    Char,
    Str,
    Int(IntTy),
    Uint(UintTy),
    Float(FloatTy),
    Option(Option<String>),
    AtomicInt(IntTy),
    AtomicUint(UintTy),
    AtomicBool,
    AtomicPtr(Option<String>),
    Tuple(Vec<Type>),
    Array(Box<Type>, String),
    Slice(Box<Type>),
    /// We shouldn't change this type because it's not trivially constructable and part of a larger
    /// type, like a tuple or array. Stores the type's path so we can write `<ty>::default()`.
    ///
    /// This is also used in the case of type aliases.
    Unchanged(Either<String, Span>),
}

const MAX_TUPLE_ARITY: usize = 3;

impl<'tcx> Type {
    fn peel_refs(&self) -> &Self {
        let mut innermost = self;
        while let Type::Ref(_, box inner_ty) = innermost {
            innermost = inner_ty;
        }
        innermost
    }

    /// Convert from a `Ty` to our `Type`. Should only be used when a HIR `Ty` isn't available
    /// because it matches on expanded type aliases.
    fn from_ty(cx: &LateContext<'tcx>, ty: Ty<'tcx>, type_is_certain: bool) -> Option<Self> {
        match ty.kind() {
            ty::Bool => Some(Self::Bool),
            ty::Char => Some(Self::Char),
            ty::Int(int_ty) => Some(Self::Int(ast_int_ty(*int_ty))),
            ty::Uint(uint_ty) => Some(Self::Uint(ast_uint_ty(*uint_ty))),
            ty::Float(float_ty) => Some(Self::Float(match float_ty {
                ty::FloatTy::F32 => FloatTy::F32,
                ty::FloatTy::F64 => FloatTy::F64,
            })),
            ty::Adt(def, args) => Some(Self::from_def_id(
                cx,
                def.did(),
                if args.is_empty() {
                    None
                } else {
                    Some(args.iter().map(|arg| arg.to_string()).join(", "))
                },
                type_is_certain,
            )),
            ty::Str => Some(Self::Str),
            ty::Array(inner_ty, len) => Some(Self::Array(
                Box::new(Self::from_ty(cx, *inner_ty, type_is_certain)?),
                len.try_to_target_usize(cx.tcx)?.to_string(),
            )),
            ty::Slice(inner_ty) => Some(Self::Slice(Box::new(Self::from_ty(cx, *inner_ty, type_is_certain)?))),
            ty::Ref(_, inner_ty, mutbl) => Some(Self::Ref(
                *mutbl,
                Box::new(Self::from_ty(cx, *inner_ty, type_is_certain)?),
            )),
            ty::Tuple(tys) if tys.len() <= MAX_TUPLE_ARITY => Some(Self::Tuple(
                tys.iter()
                    .map(|ty| Self::from_ty(cx, ty, type_is_certain))
                    .collect::<Option<Vec<_>>>()?,
            )),
            ty::Infer(infer) => Some(Self::Unchanged(Either::Left(infer.to_string()))),
            _ => None,
        }
    }

    /// Convert from a HIR `Ty` to our `Type`. Will fall back on `from_ty` on anything that must be
    /// inferred.
    ///
    /// This is preferable over `from_ty` as it will ignore type aliases as much as possible.
    fn from_hir_ty(cx: &LateContext<'tcx>, ty: &'tcx rustc_hir::Ty<'tcx>, type_is_certain: bool) -> Option<Self> {
        match ty.kind {
            TyKind::Slice(inner_ty) => Some(Self::Slice(Self::from_hir_ty(cx, inner_ty, type_is_certain)?.into())),
            TyKind::Array(inner_ty, len) => Some(Self::Array(
                Self::from_hir_ty(cx, inner_ty, type_is_certain)?.into(),
                match len {
                    ArrayLen::Infer(_, span) => snippet_opt(cx, span)?,
                    ArrayLen::Body(r#const) => snippet_opt(cx, cx.tcx.hir().opt_span(r#const.hir_id)?)?,
                },
            )),
            TyKind::Ref(_, mut_and_ty) => Some(Self::Ref(
                mut_and_ty.mutbl,
                Self::from_hir_ty(cx, mut_and_ty.ty, type_is_certain)?.into(),
            )),
            TyKind::Tup(tys) if tys.len() <= MAX_TUPLE_ARITY => Some(Self::Tuple(
                tys.iter()
                    .map(|ty| Self::from_hir_ty(cx, ty, type_is_certain))
                    .collect::<Option<Vec<_>>>()?,
            )),
            TyKind::Path(qpath) => match cx.qpath_res(&qpath, ty.hir_id) {
                Res::Def(DefKind::TyAlias, _) => Some(Self::Unchanged(Either::Right(qpath.span()))),
                Res::Def(_, def_id) => {
                    let args = match qpath {
                        QPath::Resolved(None, path) if let [.., segment] = path.segments => segment
                            .args
                            .and_then(GenericArgs::span)
                            .and_then(|span| snippet_opt(cx, span)),
                        QPath::TypeRelative(_, segment) => segment
                            .args
                            .and_then(GenericArgs::span)
                            .and_then(|span| snippet_opt(cx, span)),
                        _ => None,
                    };

                    Some(Self::from_def_id(cx, def_id, args, type_is_certain))
                },
                Res::PrimTy(prim_ty) => Some(prim_ty.into()),
                _ => None,
            },
            // Fallback on `from_ty` when we don't immediately know the type. We do this on `Default::default` and stuff
            // like `<_>::default`.
            TyKind::Infer => Self::from_ty(cx, cx.typeck_results().node_type(ty.hir_id), type_is_certain),
            _ => None,
        }
    }

    fn from_def_id(cx: &LateContext<'tcx>, def_id: DefId, args: Option<String>, type_is_certain: bool) -> Self {
        let args = if type_is_certain { None } else { args };

        if is_lang_item_or_ctor(cx, def_id, LangItem::Option) {
            Self::Option(args)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicBool) {
            Self::AtomicBool
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicI8) {
            Self::AtomicInt(IntTy::I8)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicI16) {
            Self::AtomicInt(IntTy::I16)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicI32) {
            Self::AtomicInt(IntTy::I32)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicI64) {
            Self::AtomicInt(IntTy::I64)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicIsize) {
            Self::AtomicInt(IntTy::Isize)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicPtr) {
            Self::AtomicPtr(args)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicU8) {
            Self::AtomicUint(UintTy::U8)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicU16) {
            Self::AtomicUint(UintTy::U16)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicU32) {
            Self::AtomicUint(UintTy::U32)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicU64) {
            Self::AtomicUint(UintTy::U64)
        } else if is_diagnostic_item_or_ctor(cx, def_id, sym::AtomicUsize) {
            Self::AtomicUint(UintTy::Usize)
        } else {
            Self::Unchanged(Either::Left(
                // If we don't know this type, we'll construct it with `Default::default`. This way, something like
                // `<(u32, SomeType)>::default()` will become `(0u32, SomeType::default())`.
                cx.tcx.def_path_str(def_id) + &args.map(|args| format!("::<{args}>")).unwrap_or_default(),
            ))
        }
    }

    fn display<'a>(&self, cx: &'a LateContext<'tcx>) -> TypeDisplay<'a, '_, 'tcx> {
        TypeDisplay { cx, ty: self }
    }
}

impl From<PrimTy> for Type {
    fn from(value: PrimTy) -> Self {
        match value {
            PrimTy::Int(int_ty) => Self::Int(int_ty),
            PrimTy::Uint(uint_ty) => Self::Uint(uint_ty),
            PrimTy::Float(float_ty) => Self::Float(float_ty),
            PrimTy::Str => Self::Str,
            PrimTy::Char => Self::Char,
            PrimTy::Bool => Self::Bool,
        }
    }
}

struct TypeDisplay<'a, 'b, 'tcx> {
    cx: &'a LateContext<'tcx>,
    ty: &'b Type,
}

impl Display for TypeDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { cx, ty } = self;

        let std_or_core = std_or_core(cx).unwrap_or("<_>");
        let atomic_path = format!("{std_or_core}::sync::atomic");

        match ty {
            // This will result in `&""` sometimes, but this should be ok as the compiler will always implicitly
            // dereference it anyway. Other lints will take care of this.
            Type::Ref(mutability, ty) => write!(f, "{}{}", mutability.ref_prefix_str(), TypeDisplay { cx, ty },),
            Type::Bool => write!(f, "false"),
            // Debatable on whether this is more readable or not.
            Type::Char => write!(f, "'\\x00'"),
            Type::Str => write!(f, "\"\""),
            Type::Int(int_ty) => write!(f, "0{}", int_ty.name_str()),
            Type::Uint(uint_ty) => write!(f, "0{}", uint_ty.name_str()),
            Type::Float(float_ty) => write!(f, "0.0{}", float_ty.name_str()),
            Type::Option(arg) => write!(
                f,
                "None{}",
                arg.as_ref().map(|arg| format!("::<{arg}>")).unwrap_or_default(),
            ),
            Type::AtomicInt(int_ty) => write!(
                f,
                "{atomic_path}::Atomic{}::new(0)",
                int_ty.name_str().to_ascii_uppercase(),
            ),
            Type::AtomicUint(uint_ty) => write!(
                f,
                "{atomic_path}::Atomic{}::new(0)",
                uint_ty.name_str().to_ascii_uppercase(),
            ),
            Type::AtomicBool => write!(f, "{atomic_path}::AtomicBool::new(false)"),
            Type::AtomicPtr(arg) => write!(
                f,
                "{atomic_path}::AtomicPtr::new({std_or_core}::ptr::null_mut{}())",
                arg.as_ref().map(|arg| format!("::<{arg}>")).unwrap_or_default(),
            ),
            Type::Tuple(tys) => {
                if let [ty] = tys.as_slice() {
                    write!(f, "({},)", TypeDisplay { cx, ty })
                } else {
                    write!(f, "(")?;
                    write!(
                        f,
                        "{}",
                        tys.iter().map(|ty| TypeDisplay { cx, ty }.to_string()).join(", "),
                    )?;
                    write!(f, ")")
                }
            },
            Type::Array(ty, len) => write!(f, "[{}; {len}]", TypeDisplay { cx, ty }),
            Type::Slice(ty) => write!(f, "[{}]", TypeDisplay { cx, ty }),
            // This unfortunately always has the brackets around the type, but this should be ok. We should have a lint
            // for that in the future
            Type::Unchanged(path) => match path {
                Either::Left(str) => write!(f, "<{str}>::default()"),
                Either::Right(span) => write!(f, "<{}>::default()", snippet(*cx, *span, "<ty>")),
            },
        }
    }
}
