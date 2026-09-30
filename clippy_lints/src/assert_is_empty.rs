use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::walk_span_to_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::sym;
use clippy_utils::ty::{has_debug_impl, implements_trait};
use rustc_attr_ir::lang_items::LangItem;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, GenericParamKind, HirId, ItemKind, PrimTy, UnOp, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _, impl_lint_pass};
use rustc_middle::ty::{self, GenericArgKind, IsSuggestable as _, Ty, TyCtxt};
use rustc_span::{Span, Symbol};
use std::cell::OnceCell;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks assertions that only test whether a supported value is empty.
    ///
    /// The lint handles `assert!` and `debug_assert!` calls on strings, slices, arrays, `Vec`, and
    /// the `std::collections` types `BinaryHeap`, `BTreeMap`, `BTreeSet`, `HashMap`, `HashSet`,
    /// `LinkedList`, and `VecDeque`.
    ///
    /// ### Why is this bad?
    ///
    /// A boolean assertion only reports that the emptiness check failed. It does not show what the
    /// asserted value contained.
    ///
    /// In CI or another remote test service, the failure output tells you that the value was
    /// unexpectedly empty or non-empty, but not which values were present. The next step is often
    /// to reproduce the failure locally, add temporary logging, or change the test so it exposes
    /// the value. That extra investigation can be much slower than fixing the problem from the
    /// original CI failure.
    ///
    /// The emptiness check also commonly appears before a deeper contents assertion:
    ///
    /// ```no_run
    /// # let items = vec!["baz"];
    /// assert!(!items.is_empty());
    /// assert_eq!(items[0], "bar");
    /// ```
    ///
    /// If the first assertion fails, the second assertion never runs, so the failure can hide the
    /// check that would have shown more useful context.
    ///
    /// Instead, compare the value with an empty value using `assert_eq!`, `assert_ne!`,
    /// `debug_assert_eq!`, or `debug_assert_ne!`. These macros print the asserted value on failure.
    ///
    /// ### Known problems
    ///
    /// Printing the asserted value can be undesirable outside tests, especially when the value may
    /// be very large or contain sensitive information. If the assertion failure should not reveal
    /// the value, keep the boolean assertion and allow this lint at that assertion.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// # let items = vec![1, 2, 3];
    /// assert!(items.is_empty());
    /// assert!(!items.is_empty());
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// # let items = vec![1, 2, 3];
    /// assert_eq!(items, [] as [i32; 0]);
    /// assert_ne!(items, [] as [i32; 0]);
    /// ```
    ///
    /// Collections without a compact empty literal compare against their `Default` value:
    ///
    /// ```no_run
    /// # use std::collections::HashMap;
    /// # let counts: HashMap<&str, usize> = HashMap::new();
    /// assert!(counts.is_empty());
    /// assert!(!counts.is_empty());
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// # use std::collections::HashMap;
    /// # let counts: HashMap<&str, usize> = HashMap::new();
    /// assert_eq!(counts, std::collections::HashMap::default());
    /// assert_ne!(counts, std::collections::HashMap::default());
    /// ```
    #[clippy::version = "1.98.0"]
    pub ASSERT_IS_EMPTY,
    pedantic,
    "asserting on emptiness without showing the asserted value on failure"
}

impl_lint_pass!(AssertIsEmpty => [ASSERT_IS_EMPTY]);

pub struct AssertIsEmpty {
    msrv: Msrv,

    /// Path roots the crate rebinds, computed lazily.
    rebound_path_roots: OnceCell<Vec<Symbol>>,
}

impl AssertIsEmpty {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv.into(),
            rebound_path_roots: OnceCell::new(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for AssertIsEmpty {
    /// Finds assertion conditions that only test emptiness.
    ///
    /// Matching assertions without a custom message are rewritten as equality or inequality
    /// assertions against an empty value.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // `assert!` and `debug_assert!` expand to `if`; skip other expressions before walking the
        // macro backtrace.
        if !matches!(expr.kind, ExprKind::If(..)) {
            return;
        }

        let Some((macro_name, condition, assert_span)) = assert_call(cx, expr) else {
            return;
        };
        let Some((assertion_kind, receiver)) = emptiness_assertion(condition) else {
            return;
        };
        let Some(mut rewrite) = assertion_suggestion(cx, receiver, self.msrv) else {
            return;
        };
        // A printed path may not resolve if the crate rebinds a name in it.
        if rewrite.applicability == Applicability::MachineApplicable {
            let rebound = self.rebound_path_roots.get_or_init(|| rebound_path_roots(cx.tcx));
            if mentions_any(&rewrite.empty_value, rebound) {
                rewrite.applicability = Applicability::MaybeIncorrect;
            }
        }

        emit_assertion_suggestion(
            cx,
            macro_name,
            assert_span,
            condition,
            assertion_kind,
            receiver,
            &rewrite,
        );
    }
}

/// Extracts the source-level condition from `assert!` and `debug_assert!`.
///
/// The returned macro name omits the trailing `!` so the diagnostic can build `assert_eq`,
/// `assert_ne`, `debug_assert_eq`, or `debug_assert_ne` from the original macro. Returns `None`
/// for other macros, assertions with a custom message parameter, and conditions from macro
/// expansions, where rewriting the condition span would produce confusing or invalid suggestions.
fn assert_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> Option<(&'static str, &'tcx Expr<'tcx>, Span)> {
    let macro_call = root_macro_call_first_node(cx, expr)?;
    let macro_name = match cx.tcx.get_diagnostic_name(macro_call.def_id) {
        Some(sym::assert_macro) => "assert",
        Some(sym::debug_assert_macro) => "debug_assert",
        _ => return None,
    };
    let (condition, panic_expn) = find_assert_args(cx, expr, macro_call.expn)?;
    if !panic_expn.is_default_message() {
        return None;
    }

    if condition.span.from_expansion() {
        return None;
    }

    Some((macro_name, condition, macro_call.span))
}

/// Returns the assertion kind and receiver for an emptiness predicate.
///
/// `value.is_empty()` maps to an equality assertion against an empty value. `!value.is_empty()`
/// maps to an inequality assertion. Returns `None` when the condition is neither form.
fn emptiness_assertion<'tcx>(condition: &'tcx Expr<'tcx>) -> Option<(AssertionKind, &'tcx Expr<'tcx>)> {
    if let Some(receiver) = is_empty_receiver(condition) {
        return Some((AssertionKind::Eq, receiver));
    }

    let ExprKind::Unary(UnOp::Not, inner) = condition.kind else {
        return None;
    };

    is_empty_receiver(inner).map(|receiver| (AssertionKind::Ne, receiver))
}

/// Returns the receiver when `expr` is a direct `value.is_empty()` call.
///
/// Returns `None` for other method calls, negated expressions, and non-method expressions.
fn is_empty_receiver<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::MethodCall(method, receiver, [], _) = expr.kind else {
        return None;
    };
    (method.ident.name == sym::is_empty).then_some(receiver)
}

/// Assertion macro polarity for the replacement assertion.
///
/// The variants are named after the comparison macro suffixes rather than the original predicate
/// shape because suggestions are the only consumer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AssertionKind {
    /// Use an equality assertion against an empty value.
    Eq,

    /// Use an inequality assertion against an empty value.
    Ne,
}

impl AssertionKind {
    /// Returns the assertion macro suffix for this emptiness predicate.
    ///
    /// Empty checks become equality assertions. Non-empty checks become inequality assertions.
    fn suffix(self) -> &'static str {
        match self {
            Self::Eq => "_eq",
            Self::Ne => "_ne",
        }
    }

    /// Returns the expected state named in the diagnostic.
    fn expected_state(self) -> &'static str {
        match self {
            Self::Eq => "empty",
            Self::Ne => "not empty",
        }
    }
}

/// Operands of the replacement assertion.
struct Rewrite {
    /// Number of `*` applied to the receiver, since `&C` does not compare with `C`.
    derefs: usize,

    /// Suffix appended to the receiver, e.g. `.as_slice()`.
    suffix: &'static str,

    /// Right-hand side of the comparison.
    empty_value: String,

    /// Whether the replacement assertion is known to compile.
    applicability: Applicability,
}

impl Rewrite {
    /// Compares the receiver as written against `empty_value`.
    fn direct(empty_value: impl Into<String>) -> Self {
        Self {
            derefs: 0,
            suffix: "",
            empty_value: empty_value.into(),
            applicability: Applicability::MachineApplicable,
        }
    }

    /// Compares the receiver against an empty array of `element_ty`.
    ///
    /// Typed when nameable, since a bare `[]` may not infer (e.g. for `String`); otherwise a bare
    /// `[]` as `MaybeIncorrect`.
    fn empty_array<'tcx>(cx: &LateContext<'tcx>, element_ty: Ty<'tcx>) -> Self {
        if is_nameable(cx, element_ty) {
            Self {
                applicability: canonical_root_applicability(cx, element_ty),
                ..Self::direct(format!("[] as [{element_ty}; 0]"))
            }
        } else {
            Self {
                applicability: Applicability::MaybeIncorrect,
                ..Self::direct("[]")
            }
        }
    }

    /// Like `empty_array`, through `as_slice()`.
    fn as_slice<'tcx>(cx: &LateContext<'tcx>, element_ty: Ty<'tcx>) -> Self {
        Self {
            suffix: ".as_slice()",
            ..Self::empty_array(cx, element_ty)
        }
    }
}

/// Returns whether `ty` can be written in the suggestion.
///
/// Accepts structural types, generic parameters, and standard library types. Opaque types cannot
/// be cast to, and local types may print as relative, function-scoped, or private paths. Path
/// roots are checked by `canonical_root_applicability` and `rebound_path_roots`.
fn is_nameable<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.is_suggestable(cx.tcx, false)
        && ty.walk().all(|arg| match arg.kind() {
            GenericArgKind::Type(ty) => match ty.kind() {
                ty::Adt(adt, _) => matches!(cx.tcx.crate_name(adt.did().krate), sym::std | sym::core | sym::alloc),
                ty::Bool
                | ty::Char
                | ty::Int(_)
                | ty::Uint(_)
                | ty::Float(_)
                | ty::Str
                | ty::Array(..)
                | ty::Slice(_)
                | ty::Ref(..)
                | ty::RawPtr(..)
                | ty::Tuple(_)
                | ty::Param(_) => true,
                _ => false,
            },
            GenericArgKind::Lifetime(_) | GenericArgKind::Const(_) => true,
        })
}

/// `MachineApplicable` if every standard library type in `ty` prints under `std`, `core`, or
/// `alloc`, else `MaybeIncorrect`.
///
/// A crate named by an `extern crate` item prints through it, e.g. `memory::string::String` or
/// `m::memory::string::String`, which may not resolve at the assertion.
fn canonical_root_applicability<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Applicability {
    let canonical = ty.walk().all(|arg| match arg.kind() {
        GenericArgKind::Type(ty) => match ty.kind() {
            ty::Adt(adt, _) => has_canonical_root(cx, adt.did()),
            _ => true,
        },
        GenericArgKind::Lifetime(_) | GenericArgKind::Const(_) => true,
    });
    if canonical {
        Applicability::MachineApplicable
    } else {
        Applicability::MaybeIncorrect
    }
}

/// Returns whether `def_id` prints under `std`, `core`, or `alloc`.
fn has_canonical_root(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(def_id).split("::").next(),
        Some("std" | "core" | "alloc")
    )
}

/// Builds replacement operands when the resulting assertion is useful.
///
/// The replacement assertion must compile, compare the same value, and print useful failure
/// output. Returns `None` for unsupported collection types and for element types that cannot be
/// printed and compared by the replacement assertion.
fn assertion_suggestion<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx Expr<'tcx>, msrv: Msrv) -> Option<Rewrite> {
    let receiver_ty = cx.typeck_results().expr_ty(receiver);
    let rewrite = suggestion_for_type(cx, receiver_ty, msrv)?;
    if type_is_printable_and_comparable(cx, receiver_ty.peel_refs()) {
        Some(rewrite)
    } else {
        None
    }
}

/// Returns the rewrite for this receiver type.
///
/// Arrays and borrowed vectors are compared through slices because direct comparison with `[]` does
/// not compile for those receiver types. Returns `None` when the receiver has no compact
/// empty-value comparison.
fn suggestion_for_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, msrv: Msrv) -> Option<Rewrite> {
    match ty.kind() {
        ty::Array(..) => Some(Rewrite::as_slice(cx, element_type(cx, ty)?)),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Array(..)) => {
            Some(Rewrite::as_slice(cx, element_type(cx, *inner)?))
        },
        ty::Ref(_, inner, _) if inner.is_diag_item(cx, sym::Vec) => {
            Some(Rewrite::as_slice(cx, element_type(cx, *inner)?))
        },
        _ => suggestion_for_peeled_type(cx, ty.peel_refs(), ref_depth(ty), msrv),
    }
}

/// Returns the rewrite for receivers that compare directly after peeling references.
///
/// `String` and `str` compare against `""`. Slices compare against `[]`. `Vec<T>` compares
/// against an empty array of `T`, as does `BinaryHeap<T>` through `as_slice()` since it has no
/// `PartialEq`. Other `std::collections` types compare against `::Coll::default()`, which unlike
/// `Coll::new()` allows any `Default` hasher or allocator, and the leading `::` avoids local `std`
/// items. Returns `None` for other receiver types.
fn suggestion_for_peeled_type<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    derefs: usize,
    msrv: Msrv,
) -> Option<Rewrite> {
    let diag_name = ty.opt_diag_name(cx);
    if ty.is_str() || ty.is_lang_item(cx, LangItem::String) {
        Some(Rewrite::direct("\"\""))
    } else if matches!(ty.kind(), ty::Slice(..)) {
        Some(Rewrite::direct("[]"))
    } else if diag_name == Some(sym::Vec) {
        Some(Rewrite::empty_array(cx, element_type(cx, ty)?))
    } else if diag_name == Some(sym::BinaryHeap) {
        if msrv.meets(cx, msrvs::BINARY_HEAP_AS_SLICE) {
            Some(Rewrite::as_slice(cx, element_type(cx, ty)?))
        } else {
            None
        }
    } else if let ty::Adt(adt, _) = ty.kind()
        && matches!(
            diag_name,
            Some(sym::BTreeMap | sym::BTreeSet | sym::HashMap | sym::HashSet | sym::LinkedList | sym::VecDeque)
        )
    {
        Some(Rewrite {
            derefs,
            applicability: if has_canonical_root(cx, adt.did()) {
                Applicability::MachineApplicable
            } else {
                Applicability::MaybeIncorrect
            },
            ..Rewrite::direct(format!("::{}::default()", cx.tcx.def_path_str(adt.did())))
        })
    } else {
        None
    }
}

/// Returns the number of references wrapping `ty`.
fn ref_depth(mut ty: Ty<'_>) -> usize {
    let mut depth = 0;
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
        depth += 1;
    }
    depth
}

/// Returns whether the replacement assertion has useful failure output.
///
/// Suggestions are limited to cases where the replacement assertion can both compare the value and
/// print it on failure. Strings satisfy this directly; sequence-like values require printable,
/// self-comparable elements. Other `std::collections` types must be `Debug`, `PartialEq<Self>`,
/// and `Default`. Returns `false` when any trait bound is missing.
fn type_is_printable_and_comparable<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if ty.is_str() || ty.is_lang_item(cx, LangItem::String) {
        return true;
    }

    if is_std_collection(cx, ty) {
        return has_debug_impl(cx, ty)
            && cx
                .tcx
                .get_diagnostic_item(sym::PartialEq)
                .is_some_and(|partial_eq| implements_trait(cx, ty, partial_eq, &[ty.into()]))
            && cx
                .tcx
                .get_diagnostic_item(sym::Default)
                .is_some_and(|default| implements_trait(cx, ty, default, &[]));
    }

    if let Some(element_ty) = element_type(cx, ty)
        && let Some(debug_trait) = cx.tcx.get_diagnostic_item(sym::Debug)
        && let Some(partial_eq_trait) = cx.tcx.get_diagnostic_item(sym::PartialEq)
    {
        implements_trait(cx, element_ty, debug_trait, &[])
            && implements_trait(cx, element_ty, partial_eq_trait, &[element_ty.into()])
    } else {
        false
    }
}

/// Extracts the element type from supported sequence-like values.
///
/// The element type is used both for trait checks and for the typed empty-array suggestion.
/// Returns `None` for non-sequence receiver types.
fn element_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Array(ty, _) | ty::Slice(ty) => Some(*ty),
        ty::Adt(_, args) if matches!(ty.opt_diag_name(cx), Some(sym::Vec | sym::BinaryHeap)) => args.types().next(),
        _ => None,
    }
}

/// Returns whether `ty` is a `std::collections` type compared against its `Default` value.
///
/// Excludes `BinaryHeap`, which has no `PartialEq` and goes through `as_slice()`.
fn is_std_collection<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(
        ty.opt_diag_name(cx),
        Some(sym::BTreeMap | sym::BTreeSet | sym::HashMap | sym::HashSet | sym::LinkedList | sym::VecDeque)
    )
}

/// Returns the path roots (`std`, `core`, `alloc`, primitive names) that the crate rebinds.
///
/// Bindings such as `mod std {}`, `struct i32;`, `use foo as std;`, or a glob import from another
/// crate shadow printed paths where in scope, and a renamed `extern crate` also changes `::std`.
/// Names can't be resolved at the assertion, so any binding in the crate counts.
fn rebound_path_roots(tcx: TyCtxt<'_>) -> Vec<Symbol> {
    let items = tcx.hir_crate_items(());
    let definitions = items.definitions().flat_map(|def_id| {
        let def_kind = tcx.def_kind(def_id);
        let definition = tcx
            .opt_item_name(def_id.to_def_id())
            .filter(|_| def_kind.ns() == Some(Namespace::TypeNS))
            .map(|name| (name, Res::Def(def_kind, def_id.to_def_id())));
        let type_params = tcx
            .hir_node_by_def_id(def_id)
            .generics()
            .into_iter()
            .flat_map(|generics| generics.params)
            .filter(|param| matches!(param.kind, GenericParamKind::Type { .. }))
            .map(|param| {
                (
                    param.name.ident().name,
                    Res::Def(DefKind::TyParam, param.def_id.to_def_id()),
                )
            });
        definition.into_iter().chain(type_params)
    });
    let imports = items.free_items().filter_map(|id| {
        let item = tcx.hir_item(id);
        match item.kind {
            ItemKind::ExternCrate(_, ident) => {
                let res = tcx
                    .extern_mod_stmt_cnum(item.owner_id.def_id)
                    .map_or(Res::Err, |cnum| Res::Def(DefKind::Mod, cnum.as_def_id()));
                Some((ident.name, res))
            },
            ItemKind::Use(path, UseKind::Single(ident)) => Some((ident.name, path.res.type_ns?)),
            _ => None,
        }
    });
    // Globs over local modules only bring in names already found above.
    let glob_imports = items.free_items().flat_map(|id| {
        let children = match tcx.hir_item(id).kind {
            ItemKind::Use(path, UseKind::Glob) => match path.res.type_ns {
                Some(Res::Def(DefKind::Mod | DefKind::Enum, def_id)) if !def_id.is_local() => {
                    tcx.module_children(def_id)
                },
                _ => &[],
            },
            _ => &[],
        };
        children
            .iter()
            .filter(|child| child.vis.is_public() && child.res.ns() == Some(Namespace::TypeNS))
            .map(|child| (child.ident.name, child.res.map_id(|never| -> HirId { never })))
    });

    let mut roots: Vec<Symbol> = definitions
        .chain(imports)
        .chain(glob_imports)
        .filter(|&(name, res)| rebinds_path_root(tcx, name, res))
        .map(|(name, _)| name)
        .collect();
    roots.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    roots.dedup();
    roots
}

/// Returns whether binding `name` to `res` shadows `name` as a path root.
///
/// Not if it binds a crate to its own name (`extern crate alloc;`), or is a module named like a
/// primitive (`use std::str;`), which types resolve past.
fn rebinds_path_root(tcx: TyCtxt<'_>, name: Symbol, res: Res) -> bool {
    if matches!(name, sym::std | sym::core | sym::alloc) {
        !matches!(res, Res::Def(DefKind::Mod, def_id) if def_id.is_crate_root() && tcx.crate_name(def_id.krate) == name)
    } else if PrimTy::from_name(name).is_some() {
        !matches!(res, Res::Def(DefKind::Mod, _)) && !matches!(res, Res::PrimTy(prim) if prim.name() == name)
    } else {
        false
    }
}

/// Returns whether `code` contains any of `names` as a whole word.
fn mentions_any(code: &str, names: &[Symbol]) -> bool {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| names.iter().any(|name| name.as_str() == word))
}

/// Emits the rewrite from a boolean assertion to a comparison assertion.
fn emit_assertion_suggestion(
    cx: &LateContext<'_>,
    macro_name: &str,
    assert_span: Span,
    condition: &Expr<'_>,
    assertion_kind: AssertionKind,
    receiver: &Expr<'_>,
    rewrite: &Rewrite,
) {
    let mut applicability = rewrite.applicability;
    let mut receiver_snip = Sugg::hir_with_context(cx, receiver, assert_span.ctxt(), "..", &mut applicability);
    for _ in 0..rewrite.derefs {
        receiver_snip = receiver_snip.deref();
    }
    // A whole macro argument only needs parentheses before a method call.
    let receiver_snip = if rewrite.suffix.is_empty() {
        receiver_snip.to_string()
    } else {
        format!("{}{}", receiver_snip.maybe_paren(), rewrite.suffix)
    };
    let empty_value = &rewrite.empty_value;
    let assertion_suffix = assertion_kind.suffix();
    let expected_state = assertion_kind.expected_state();

    span_lint_and_then(
        cx,
        ASSERT_IS_EMPTY,
        assert_span,
        format!("used `{macro_name}!` to check that a value is {expected_state}"),
        |diag| {
            let macro_name_span = cx.sess().source_map().span_until_char(assert_span, '!');
            let condition_span = walk_span_to_context(condition.span, assert_span.ctxt()).unwrap_or(condition.span);

            diag.multipart_suggestion(
                format!("use `{macro_name}{assertion_suffix}!` to show the value on failure"),
                vec![
                    (macro_name_span.shrink_to_hi(), assertion_suffix.to_string()),
                    (condition_span, format!("{receiver_snip}, {empty_value}")),
                ],
                applicability,
            );
        },
    );
}
