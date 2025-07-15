#![feature(
    if_let_guard,
    macro_metavar_expr_concat,
    never_type,
    rustc_private,
    unwrap_infallible
)]
#![allow(
    clippy::missing_docs_in_private_items,
    clippy::must_use_candidate,
    rustc::diagnostic_outside_of_impl,
    rustc::untranslatable_diagnostic,
    clippy::literal_string_with_formatting_args
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications,
    rustc::internal
)]

extern crate rustc_abi;
extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_trait_selection;
extern crate smallvec;

#[macro_use]
extern crate declare_clippy_lint;

pub mod declared_lints;

mod option_map_unwrap_or;
mod result_map_or_else_none;
mod utils;

// begin lints modules, do not remove this comment, it's used in `update_lints`
mod bind_instead_of_map;
mod bytecount;
mod bytes_count_to_len;
mod bytes_nth;
mod case_sensitive_file_extension_comparisons;
mod chars_cmp;
mod clear_with_drain;
mod clone_on_copy;
mod clone_on_ref_ptr;
mod cloned_instead_of_copied;
mod collapsible_str_replace;
mod double_ended_iterator_last;
mod drain_collect;
mod err_expect;
mod expect_fun_call;
mod extend_with_drain;
mod filetype_is_file;
mod filter_map;
mod filter_map_bool_then;
mod filter_map_identity;
mod filter_map_next;
mod filter_next;
mod flat_map_identity;
mod flat_map_option;
mod format_collect;
mod from_iter_instead_of_collect;
mod get_first;
mod get_last_with_len;
mod get_unwrap;
mod implicit_clone;
mod inefficient_to_string;
mod inspect_for_each;
mod into_iter_on_ref;
mod io_other_error;
mod ip_constant;
mod is_digit_ascii_radix;
mod is_empty;
mod iter_cloned_collect;
mod iter_count;
mod iter_filter;
mod iter_kv_map;
mod iter_next_slice;
mod iter_nth;
mod iter_nth_zero;
mod iter_on_single_or_empty_collections;
mod iter_out_of_bounds;
mod iter_overeager_cloned;
mod iter_skip_next;
mod iter_skip_zero;
mod iter_with_drain;
mod iterator_step_by_zero;
mod join_absolute_paths;
mod manual_c_str_literals;
mod manual_contains;
mod manual_inspect;
mod manual_is_variant_and;
mod manual_next_back;
mod manual_ok_or;
mod manual_repeat_n;
mod manual_saturating_arithmetic;
mod manual_str_repeat;
mod manual_try_fold;
mod map_all_any_identity;
mod map_clone;
mod map_collect_result_unit;
mod map_err_ignore;
mod map_flatten;
mod map_identity;
mod map_unwrap_or;
mod map_with_unused_argument_over_ranges;
mod mut_mutex_lock;
mod needless_as_bytes;
mod needless_character_iteration;
mod needless_collect;
mod needless_option_as_deref;
mod needless_option_take;
mod no_effect_replace;
mod obfuscated_if_else;
mod ok_expect;
mod open_options;
mod option_as_ref_cloned;
mod option_as_ref_deref;
mod option_map_or_none;
mod or_fun_call;
mod or_then_unwrap;
mod path_buf_push_overwrite;
mod path_ends_with_ext;
mod range_zip_with_len;
mod read_line_without_trim;
mod readonly_write_lock;
mod redundant_as_str;
mod repeat_once;
mod return_and_then;
mod search_is_some;
mod seek_from_current;
mod seek_to_start_instead_of_rewind;
mod single_char_add_str;
mod skip_while_next;
mod sliced_string_as_bytes;
mod stable_sort_primitive;
mod str_split;
mod str_splitn;
mod string_extend_chars;
mod string_lit_chars_any;
mod suspicious_command_arg_space;
mod suspicious_map;
mod suspicious_splitn;
mod suspicious_to_owned;
mod swap_with_temporary;
mod type_id_on_box;
mod unbuffered_bytes;
mod uninit_assumed_init;
mod unit_hash;
mod unnecessary_fallible_conversions;
mod unnecessary_filter_map;
mod unnecessary_first_then_check;
mod unnecessary_fold;
mod unnecessary_get_then_check;
mod unnecessary_join;
mod unnecessary_lazy_eval;
mod unnecessary_literal_unwrap;
mod unnecessary_map_or;
mod unnecessary_min_or_max;
mod unnecessary_result_map_or_else;
mod unnecessary_sort_by;
mod unnecessary_to_owned;
mod unwrap_expect_used;
mod useless_asref;
mod useless_nonzero_new_unchecked;
mod vec_resize_to_zero;
mod verbose_file_reads;
mod waker_clone_wake;
mod wrong_self_convention;
mod zst_offset;
// end lints modules, do not remove this comment, it's used in `update_lints`

use clippy_config::Conf;
use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use clippy_utils::macros::FormatArgsStorage;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::ty::{contains_ty_adt_constructor_opaque, implements_trait, is_copy, is_type_diagnostic_item};
use clippy_utils::{contains_return, is_bool, is_trait_method, iter_input_pats, peel_blocks, return_ty, sym};
pub use path_ends_with_ext::DEFAULT_ALLOWED_DOTFILES;
use rustc_abi::ExternAbi;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir as hir;
use rustc_hir::{Expr, ExprKind, Node, Stmt, StmtKind, TraitItem, TraitItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, TraitRef, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol, kw};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for methods that should live in a trait
    /// implementation of a `std` trait (see [llogiq's blog
    /// post](http://llogiq.github.io/2015/07/30/traits.html) for further
    /// information) instead of an inherent implementation.
    ///
    /// ### Why is this bad?
    /// Implementing the traits improve ergonomics for users of
    /// the code, often with very little cost. Also people seeing a `mul(...)`
    /// method
    /// may expect `*` to work equally, so you should have good reason to disappoint
    /// them.
    ///
    /// ### Example
    /// ```no_run
    /// struct X;
    /// impl X {
    ///     fn add(&self, other: &X) -> X {
    ///         // ..
    /// # X
    ///     }
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub SHOULD_IMPLEMENT_TRAIT,
    style,
    "defining a method that should be implementing a std trait"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `new` not returning a type that contains `Self`.
    ///
    /// ### Why is this bad?
    /// As a convention, `new` methods are used to make a new
    /// instance of a type.
    ///
    /// ### Example
    /// In an impl block:
    /// ```no_run
    /// # struct Foo;
    /// # struct NotAFoo;
    /// impl Foo {
    ///     fn new() -> NotAFoo {
    /// # NotAFoo
    ///     }
    /// }
    /// ```
    ///
    /// ```no_run
    /// # struct Foo;
    /// struct Bar(Foo);
    /// impl Foo {
    ///     // Bad. The type name must contain `Self`
    ///     fn new() -> Bar {
    /// # Bar(Foo)
    ///     }
    /// }
    /// ```
    ///
    /// ```no_run
    /// # struct Foo;
    /// # struct FooError;
    /// impl Foo {
    ///     // Good. Return type contains `Self`
    ///     fn new() -> Result<Foo, FooError> {
    /// # Ok(Foo)
    ///     }
    /// }
    /// ```
    ///
    /// Or in a trait definition:
    /// ```no_run
    /// pub trait Trait {
    ///     // Bad. The type name must contain `Self`
    ///     fn new();
    /// }
    /// ```
    ///
    /// ```no_run
    /// pub trait Trait {
    ///     // Good. Return type contains `Self`
    ///     fn new() -> Self;
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NEW_RET_NO_SELF,
    style,
    "not returning type containing `Self` in a `new` method"
}

#[expect(clippy::struct_excessive_bools)]
struct Methods {
    avoid_breaking_exported_api: bool,
    msrv: Msrv,
    allow_expect_in_tests: bool,
    allow_unwrap_in_tests: bool,
    allow_expect_in_consts: bool,
    allow_unwrap_in_consts: bool,
    allowed_dotfiles: FxHashSet<&'static str>,
    format_args: FormatArgsStorage,
}

impl Methods {
    fn new(conf: &'static Conf, format_args: FormatArgsStorage) -> Self {
        let mut allowed_dotfiles: FxHashSet<_> = conf.allowed_dotfiles.iter().map(|s| &**s).collect();
        allowed_dotfiles.extend(DEFAULT_ALLOWED_DOTFILES);

        Self {
            avoid_breaking_exported_api: conf.avoid_breaking_exported_api,
            msrv: conf.msrv,
            allow_expect_in_tests: conf.allow_expect_in_tests,
            allow_unwrap_in_tests: conf.allow_unwrap_in_tests,
            allow_expect_in_consts: conf.allow_expect_in_consts,
            allow_unwrap_in_consts: conf.allow_unwrap_in_consts,
            allowed_dotfiles,
            format_args,
        }
    }
}

impl_lint_pass!(Methods => [
    unwrap_expect_used::UNWRAP_USED,
    unwrap_expect_used::EXPECT_USED,
    SHOULD_IMPLEMENT_TRAIT,
    wrong_self_convention::WRONG_SELF_CONVENTION,
    ok_expect::OK_EXPECT,
    or_fun_call::UNWRAP_OR_DEFAULT,
    map_unwrap_or::MAP_UNWRAP_OR,
    option_map_or_none::RESULT_MAP_OR_INTO_OPTION,
    option_map_or_none::OPTION_MAP_OR_NONE,
    bind_instead_of_map::BIND_INSTEAD_OF_MAP,
    or_fun_call::OR_FUN_CALL,
    or_then_unwrap::OR_THEN_UNWRAP,
    expect_fun_call::EXPECT_FUN_CALL,
    chars_cmp::CHARS_NEXT_CMP,
    chars_cmp::CHARS_LAST_CMP,
    clone_on_copy::CLONE_ON_COPY,
    clone_on_ref_ptr::CLONE_ON_REF_PTR,
    collapsible_str_replace::COLLAPSIBLE_STR_REPLACE,
    is_empty::CONST_IS_EMPTY,
    iter_overeager_cloned::ITER_OVEREAGER_CLONED,
    cloned_instead_of_copied::CLONED_INSTEAD_OF_COPIED,
    flat_map_option::FLAT_MAP_OPTION,
    inefficient_to_string::INEFFICIENT_TO_STRING,
    NEW_RET_NO_SELF,
    single_char_add_str::SINGLE_CHAR_ADD_STR,
    search_is_some::SEARCH_IS_SOME,
    filter_next::FILTER_NEXT,
    skip_while_next::SKIP_WHILE_NEXT,
    filter_map_identity::FILTER_MAP_IDENTITY,
    map_identity::MAP_IDENTITY,
    filter_map::MANUAL_FILTER_MAP,
    filter_map::MANUAL_FIND_MAP,
    filter_map::OPTION_FILTER_MAP,
    filter_map_next::FILTER_MAP_NEXT,
    flat_map_identity::FLAT_MAP_IDENTITY,
    map_flatten::MAP_FLATTEN,
    iterator_step_by_zero::ITERATOR_STEP_BY_ZERO,
    iter_next_slice::ITER_NEXT_SLICE,
    iter_count::ITER_COUNT,
    iter_nth::ITER_NTH,
    iter_nth_zero::ITER_NTH_ZERO,
    bytes_nth::BYTES_NTH,
    iter_skip_next::ITER_SKIP_NEXT,
    get_unwrap::GET_UNWRAP,
    get_last_with_len::GET_LAST_WITH_LEN,
    string_extend_chars::STRING_EXTEND_CHARS,
    iter_cloned_collect::ITER_CLONED_COLLECT,
    iter_with_drain::ITER_WITH_DRAIN,
    type_id_on_box::TYPE_ID_ON_BOX,
    useless_asref::USELESS_ASREF,
    unnecessary_fold::UNNECESSARY_FOLD,
    unnecessary_filter_map::UNNECESSARY_FILTER_MAP,
    unnecessary_filter_map::UNNECESSARY_FIND_MAP,
    into_iter_on_ref::INTO_ITER_ON_REF,
    suspicious_map::SUSPICIOUS_MAP,
    uninit_assumed_init::UNINIT_ASSUMED_INIT,
    manual_saturating_arithmetic::MANUAL_SATURATING_ARITHMETIC,
    zst_offset::ZST_OFFSET,
    filetype_is_file::FILETYPE_IS_FILE,
    option_as_ref_deref::OPTION_AS_REF_DEREF,
    unnecessary_lazy_eval::UNNECESSARY_LAZY_EVALUATIONS,
    map_collect_result_unit::MAP_COLLECT_RESULT_UNIT,
    from_iter_instead_of_collect::FROM_ITER_INSTEAD_OF_COLLECT,
    inspect_for_each::INSPECT_FOR_EACH,
    implicit_clone::IMPLICIT_CLONE,
    suspicious_to_owned::SUSPICIOUS_TO_OWNED,
    suspicious_splitn::SUSPICIOUS_SPLITN,
    manual_str_repeat::MANUAL_STR_REPEAT,
    extend_with_drain::EXTEND_WITH_DRAIN,
    str_splitn::MANUAL_SPLIT_ONCE,
    str_splitn::NEEDLESS_SPLITN,
    unnecessary_to_owned::UNNECESSARY_TO_OWNED,
    unnecessary_join::UNNECESSARY_JOIN,
    err_expect::ERR_EXPECT,
    needless_option_as_deref::NEEDLESS_OPTION_AS_DEREF,
    is_digit_ascii_radix::IS_DIGIT_ASCII_RADIX,
    needless_option_take::NEEDLESS_OPTION_TAKE,
    no_effect_replace::NO_EFFECT_REPLACE,
    obfuscated_if_else::OBFUSCATED_IF_ELSE,
    iter_on_single_or_empty_collections::ITER_ON_SINGLE_ITEMS,
    iter_on_single_or_empty_collections::ITER_ON_EMPTY_COLLECTIONS,
    bytecount::NAIVE_BYTECOUNT,
    bytes_count_to_len::BYTES_COUNT_TO_LEN,
    case_sensitive_file_extension_comparisons::CASE_SENSITIVE_FILE_EXTENSION_COMPARISONS,
    get_first::GET_FIRST,
    manual_ok_or::MANUAL_OK_OR,
    map_clone::MAP_CLONE,
    map_err_ignore::MAP_ERR_IGNORE,
    mut_mutex_lock::MUT_MUTEX_LOCK,
    open_options::NONSENSICAL_OPEN_OPTIONS,
    open_options::SUSPICIOUS_OPEN_OPTIONS,
    path_buf_push_overwrite::PATH_BUF_PUSH_OVERWRITE,
    range_zip_with_len::RANGE_ZIP_WITH_LEN,
    repeat_once::REPEAT_ONCE,
    stable_sort_primitive::STABLE_SORT_PRIMITIVE,
    unit_hash::UNIT_HASH,
    read_line_without_trim::READ_LINE_WITHOUT_TRIM,
    unnecessary_sort_by::UNNECESSARY_SORT_BY,
    vec_resize_to_zero::VEC_RESIZE_TO_ZERO,
    verbose_file_reads::VERBOSE_FILE_READS,
    iter_kv_map::ITER_KV_MAP,
    seek_from_current::SEEK_FROM_CURRENT,
    seek_to_start_instead_of_rewind::SEEK_TO_START_INSTEAD_OF_REWIND,
    needless_collect::NEEDLESS_COLLECT,
    suspicious_command_arg_space::SUSPICIOUS_COMMAND_ARG_SPACE,
    clear_with_drain::CLEAR_WITH_DRAIN,
    manual_next_back::MANUAL_NEXT_BACK,
    unnecessary_literal_unwrap::UNNECESSARY_LITERAL_UNWRAP,
    drain_collect::DRAIN_COLLECT,
    manual_try_fold::MANUAL_TRY_FOLD,
    format_collect::FORMAT_COLLECT,
    string_lit_chars_any::STRING_LIT_CHARS_ANY,
    iter_skip_zero::ITER_SKIP_ZERO,
    filter_map_bool_then::FILTER_MAP_BOOL_THEN,
    readonly_write_lock::READONLY_WRITE_LOCK,
    iter_out_of_bounds::ITER_OUT_OF_BOUNDS,
    path_ends_with_ext::PATH_ENDS_WITH_EXT,
    redundant_as_str::REDUNDANT_AS_STR,
    waker_clone_wake::WAKER_CLONE_WAKE,
    unnecessary_fallible_conversions::UNNECESSARY_FALLIBLE_CONVERSIONS,
    join_absolute_paths::JOIN_ABSOLUTE_PATHS,
    filter_map::RESULT_FILTER_MAP,
    iter_filter::ITER_FILTER_IS_SOME,
    iter_filter::ITER_FILTER_IS_OK,
    manual_is_variant_and::MANUAL_IS_VARIANT_AND,
    str_split::STR_SPLIT_AT_NEWLINE,
    option_as_ref_cloned::OPTION_AS_REF_CLONED,
    unnecessary_result_map_or_else::UNNECESSARY_RESULT_MAP_OR_ELSE,
    manual_c_str_literals::MANUAL_C_STR_LITERALS,
    unnecessary_get_then_check::UNNECESSARY_GET_THEN_CHECK,
    unnecessary_first_then_check::UNNECESSARY_FIRST_THEN_CHECK,
    needless_character_iteration::NEEDLESS_CHARACTER_ITERATION,
    manual_inspect::MANUAL_INSPECT,
    unnecessary_min_or_max::UNNECESSARY_MIN_OR_MAX,
    needless_as_bytes::NEEDLESS_AS_BYTES,
    map_all_any_identity::MAP_ALL_ANY_IDENTITY,
    map_with_unused_argument_over_ranges::MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
    unnecessary_map_or::UNNECESSARY_MAP_OR,
    double_ended_iterator_last::DOUBLE_ENDED_ITERATOR_LAST,
    useless_nonzero_new_unchecked::USELESS_NONZERO_NEW_UNCHECKED,
    manual_repeat_n::MANUAL_REPEAT_N,
    sliced_string_as_bytes::SLICED_STRING_AS_BYTES,
    return_and_then::RETURN_AND_THEN,
    unbuffered_bytes::UNBUFFERED_BYTES,
    manual_contains::MANUAL_CONTAINS,
    io_other_error::IO_OTHER_ERROR,
    swap_with_temporary::SWAP_WITH_TEMPORARY,
    ip_constant::IP_CONSTANT,
    iter_overeager_cloned::REDUNDANT_ITER_CLONED,
]);

/// Extracts a method call name, args, and `Span` of the method name.
/// This ensures that neither the receiver nor any of the arguments
/// come from expansion.
fn method_call<'tcx>(recv: &'tcx Expr<'tcx>) -> Option<(Symbol, &'tcx Expr<'tcx>, &'tcx [Expr<'tcx>], Span, Span)> {
    if let ExprKind::MethodCall(path, receiver, args, call_span) = recv.kind
        && !args.iter().any(|e| e.span.from_expansion())
        && !receiver.span.from_expansion()
    {
        Some((path.ident.name, receiver, args, path.ident.span, call_span))
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for Methods {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        self.check_methods(cx, expr);

        match expr.kind {
            ExprKind::Call(func, args) => {
                from_iter_instead_of_collect::check(cx, expr, args, func);
                unnecessary_fallible_conversions::check_function(cx, expr, func);
                manual_c_str_literals::check(cx, expr, func, args, self.msrv);
                useless_nonzero_new_unchecked::check(cx, expr, func, args, self.msrv);
                io_other_error::check(cx, expr, func, args, self.msrv);
                swap_with_temporary::check(cx, expr, func, args);
                ip_constant::check(cx, expr, func, args);
            },
            ExprKind::MethodCall(method_call, receiver, args, _) => {
                let method_span = method_call.ident.span;
                or_fun_call::check(cx, expr, method_span, method_call.ident.name, receiver, args);
                expect_fun_call::check(
                    cx,
                    &self.format_args,
                    expr,
                    method_span,
                    method_call.ident.name,
                    receiver,
                    args,
                );
                clone_on_copy::check(cx, expr, method_call.ident.name, receiver, args);
                clone_on_ref_ptr::check(cx, expr, method_call.ident.name, receiver, args);
                inefficient_to_string::check(cx, expr, method_call.ident.name, receiver, args);
                single_char_add_str::check(cx, expr, receiver, args);
                into_iter_on_ref::check(cx, expr, method_span, method_call.ident.name, receiver);
                unnecessary_to_owned::check(cx, expr, method_call.ident.name, receiver, args, self.msrv);
            },
            ExprKind::Binary(op, lhs, rhs) if op.node == hir::BinOpKind::Eq || op.node == hir::BinOpKind::Ne => {
                let mut info = BinaryExprInfo {
                    expr,
                    chain: lhs,
                    other: rhs,
                    eq: op.node == hir::BinOpKind::Eq,
                };
                lint_binary_expr_with_method_call(cx, &mut info);
            },
            _ => (),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx hir::ImplItem<'_>) {
        if impl_item.span.in_external_macro(cx.sess().source_map()) {
            return;
        }
        let name = impl_item.ident.name;
        let parent = cx.tcx.hir_get_parent_item(impl_item.hir_id()).def_id;
        let item = cx.tcx.hir_expect_item(parent);
        let self_ty = cx.tcx.type_of(item.owner_id).instantiate_identity();

        let implements_trait = matches!(item.kind, hir::ItemKind::Impl(hir::Impl { of_trait: Some(_), .. }));
        if let hir::ImplItemKind::Fn(ref sig, id) = impl_item.kind {
            let method_sig = cx.tcx.fn_sig(impl_item.owner_id).instantiate_identity();
            let method_sig = cx.tcx.instantiate_bound_regions_with_erased(method_sig);
            let first_arg_ty_opt = method_sig.inputs().iter().next().copied();
            // if this impl block implements a trait, lint in trait definition instead
            if !implements_trait && cx.effective_visibilities.is_exported(impl_item.owner_id.def_id) {
                // check missing trait implementations
                for method_config in &TRAIT_METHODS {
                    if name == method_config.method_name
                        && sig.decl.inputs.len() == method_config.param_count
                        && method_config.output_type.matches(&sig.decl.output)
                        // in case there is no first arg, since we already have checked the number of arguments
                        // it's should be always true
                        && first_arg_ty_opt.is_none_or(|first_arg_ty| method_config
                            .self_kind.matches(cx, self_ty, first_arg_ty)
                            )
                        && fn_header_equals(method_config.fn_header, sig.header)
                        && method_config.lifetime_param_cond(impl_item)
                    {
                        span_lint_and_help(
                            cx,
                            SHOULD_IMPLEMENT_TRAIT,
                            impl_item.span,
                            format!(
                                "method `{}` can be confused for the standard trait method `{}::{}`",
                                method_config.method_name, method_config.trait_name, method_config.method_name
                            ),
                            None,
                            format!(
                                "consider implementing the trait `{}` or choosing a less ambiguous method name",
                                method_config.trait_name
                            ),
                        );
                    }
                }
            }

            if sig.decl.implicit_self.has_implicit_self()
                && !(self.avoid_breaking_exported_api
                    && cx.effective_visibilities.is_exported(impl_item.owner_id.def_id))
                && let Some(first_arg) = iter_input_pats(sig.decl, cx.tcx.hir_body(id)).next()
                && let Some(first_arg_ty) = first_arg_ty_opt
            {
                wrong_self_convention::check(
                    cx,
                    name,
                    self_ty,
                    first_arg_ty,
                    first_arg.pat.span,
                    implements_trait,
                    false,
                );
            }
        }

        // if this impl block implements a trait, lint in trait definition instead
        if implements_trait {
            return;
        }

        if let hir::ImplItemKind::Fn(_, _) = impl_item.kind {
            let ret_ty = return_ty(cx, impl_item.owner_id);

            if contains_ty_adt_constructor_opaque(cx, ret_ty, self_ty) {
                return;
            }

            if name == sym::new && ret_ty != self_ty {
                span_lint(
                    cx,
                    NEW_RET_NO_SELF,
                    impl_item.span,
                    "methods called `new` usually return `Self`",
                );
            }
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'_>) {
        if item.span.in_external_macro(cx.tcx.sess.source_map()) {
            return;
        }

        if let TraitItemKind::Fn(ref sig, _) = item.kind
            && sig.decl.implicit_self.has_implicit_self()
            && let Some(first_arg_hir_ty) = sig.decl.inputs.first()
            && let Some(&first_arg_ty) = cx
                .tcx
                .fn_sig(item.owner_id)
                .instantiate_identity()
                .inputs()
                .skip_binder()
                .first()
        {
            let self_ty = TraitRef::identity(cx.tcx, item.owner_id.to_def_id()).self_ty();
            wrong_self_convention::check(
                cx,
                item.ident.name,
                self_ty,
                first_arg_ty,
                first_arg_hir_ty.span,
                false,
                true,
            );
        }

        if item.ident.name == sym::new
            && let TraitItemKind::Fn(_, _) = item.kind
            && let ret_ty = return_ty(cx, item.owner_id)
            && let self_ty = TraitRef::identity(cx.tcx, item.owner_id.to_def_id()).self_ty()
            && !ret_ty.contains(self_ty)
        {
            span_lint(
                cx,
                NEW_RET_NO_SELF,
                item.span,
                "methods called `new` usually return `Self`",
            );
        }
    }
}

impl Methods {
    #[allow(clippy::too_many_lines)]
    fn check_methods<'tcx>(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Handle method calls whose receiver and arguments may not come from expansion
        if let Some((name, recv, args, span, call_span)) = method_call(expr) {
            match (name, args) {
                (
                    sym::add | sym::offset | sym::sub | sym::wrapping_offset | sym::wrapping_add | sym::wrapping_sub,
                    [_arg],
                ) => {
                    zst_offset::check(cx, expr, recv);
                },
                (sym::all, [arg]) => {
                    needless_character_iteration::check(cx, expr, recv, arg, true);
                    match method_call(recv) {
                        Some((sym::cloned, recv2, [], _, _)) => {
                            iter_overeager_cloned::check(
                                cx,
                                expr,
                                recv,
                                recv2,
                                iter_overeager_cloned::Op::NeedlessMove(arg),
                                false,
                            );
                        },
                        Some((sym::map, _, [map_arg], _, map_call_span)) => {
                            map_all_any_identity::check(cx, expr, recv, map_call_span, map_arg, call_span, arg, "all");
                        },
                        _ => {},
                    }
                },
                (sym::and_then, [arg]) => {
                    let biom_option_linted = bind_instead_of_map::check_and_then_some(cx, expr, recv, arg);
                    let biom_result_linted = bind_instead_of_map::check_and_then_ok(cx, expr, recv, arg);
                    if !biom_option_linted && !biom_result_linted {
                        let ule_and_linted = unnecessary_lazy_eval::check(cx, expr, recv, arg, "and");
                        if !ule_and_linted {
                            return_and_then::check(cx, expr, recv, arg);
                        }
                    }
                },
                (sym::any, [arg]) => {
                    needless_character_iteration::check(cx, expr, recv, arg, false);
                    match method_call(recv) {
                        Some((sym::cloned, recv2, [], _, _)) => iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::NeedlessMove(arg),
                            false,
                        ),
                        Some((sym::chars, recv, _, _, _))
                            if let ExprKind::Closure(arg) = arg.kind
                                && let body = cx.tcx.hir_body(arg.body)
                                && let [param] = body.params =>
                        {
                            string_lit_chars_any::check(cx, expr, recv, param, peel_blocks(body.value), self.msrv);
                        },
                        Some((sym::map, _, [map_arg], _, map_call_span)) => {
                            map_all_any_identity::check(cx, expr, recv, map_call_span, map_arg, call_span, arg, "any");
                        },
                        Some((sym::iter, iter_recv, ..)) => {
                            manual_contains::check(cx, expr, iter_recv, arg);
                        },
                        _ => {},
                    }
                },
                (sym::arg, [arg]) => {
                    suspicious_command_arg_space::check(cx, recv, arg, span);
                },
                (sym::as_deref | sym::as_deref_mut, []) => {
                    needless_option_as_deref::check(cx, expr, recv, name);
                },
                (sym::as_bytes, []) => {
                    if let Some((sym::as_str, recv, [], as_str_span, _)) = method_call(recv) {
                        redundant_as_str::check(cx, expr, recv, as_str_span, span);
                    }
                    sliced_string_as_bytes::check(cx, expr, recv);
                },
                (sym::as_mut | sym::as_ref, []) => useless_asref::check(cx, expr, name, recv),
                (sym::as_ptr, []) => manual_c_str_literals::check_as_ptr(cx, expr, recv, self.msrv),
                (sym::assume_init, []) => uninit_assumed_init::check(cx, expr, recv),
                (sym::bytes, []) => unbuffered_bytes::check(cx, expr, recv),
                (sym::cloned, []) => {
                    cloned_instead_of_copied::check(cx, expr, recv, span, self.msrv);
                    option_as_ref_cloned::check(cx, recv, span);
                },
                (sym::collect, []) if is_trait_method(cx, expr, sym::Iterator) => {
                    needless_collect::check(cx, span, expr, recv, call_span);
                    match method_call(recv) {
                        Some((name @ (sym::cloned | sym::copied), recv2, [], _, _)) => {
                            iter_cloned_collect::check(cx, name, expr, recv2);
                        },
                        Some((sym::map, m_recv, [m_arg], m_ident_span, _)) => {
                            map_collect_result_unit::check(cx, expr, m_recv, m_arg);
                            format_collect::check(cx, expr, m_arg, m_ident_span);
                        },
                        Some((sym::take, take_self_arg, [take_arg], _, _)) => {
                            if self.msrv.meets(cx, msrvs::STR_REPEAT) {
                                manual_str_repeat::check(cx, expr, recv, take_self_arg, take_arg);
                            }
                        },
                        Some((sym::drain, recv, args, ..)) => {
                            drain_collect::check(cx, args, expr, recv);
                        },
                        _ => {},
                    }
                },
                (sym::count, []) if is_trait_method(cx, expr, sym::Iterator) => match method_call(recv) {
                    Some((sym::cloned, recv2, [], _, _)) => {
                        iter_overeager_cloned::check(cx, expr, recv, recv2, iter_overeager_cloned::Op::RmCloned, false);
                    },
                    Some((name2 @ (sym::into_iter | sym::iter | sym::iter_mut), recv2, [], _, _)) => {
                        iter_count::check(cx, expr, recv2, name2);
                    },
                    Some((sym::map, _, [arg], _, _)) => suspicious_map::check(cx, expr, recv, arg),
                    Some((sym::filter, recv2, [arg], _, _)) => bytecount::check(cx, expr, recv2, arg),
                    Some((sym::bytes, recv2, [], _, _)) => bytes_count_to_len::check(cx, expr, recv, recv2),
                    _ => {},
                },
                (sym::min | sym::max, [arg]) => {
                    unnecessary_min_or_max::check(cx, expr, name, recv, arg);
                },
                (sym::drain, ..) => {
                    if let Node::Stmt(Stmt { hir_id: _, kind, .. }) = cx.tcx.parent_hir_node(expr.hir_id)
                        && matches!(kind, StmtKind::Semi(_))
                        && args.len() <= 1
                    {
                        clear_with_drain::check(cx, expr, recv, span, args.first());
                    } else if let [arg] = args {
                        iter_with_drain::check(cx, expr, recv, span, arg);
                    }
                },
                (sym::ends_with, [arg]) => {
                    if let ExprKind::MethodCall(.., span) = expr.kind {
                        case_sensitive_file_extension_comparisons::check(cx, expr, span, recv, arg, self.msrv);
                    }
                    path_ends_with_ext::check(cx, recv, arg, expr, self.msrv, &self.allowed_dotfiles);
                },
                (sym::expect, [_]) => {
                    match method_call(recv) {
                        Some((sym::ok, recv, [], _, _)) => ok_expect::check(cx, expr, recv),
                        Some((sym::err, recv, [], err_span, _)) => {
                            err_expect::check(cx, expr, recv, span, err_span, self.msrv);
                        },
                        _ => {},
                    }
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::expect_err, [_]) | (sym::unwrap_err | sym::unwrap_unchecked | sym::unwrap_err_unchecked, []) => {
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::extend, [arg]) => {
                    string_extend_chars::check(cx, expr, recv, arg);
                    extend_with_drain::check(cx, expr, recv, arg);
                },
                (sym::filter, [arg]) => {
                    if let Some((sym::cloned, recv2, [], _span2, _)) = method_call(recv) {
                        // if `arg` has side-effect, the semantic will change
                        iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::FixClosure(name, arg),
                            false,
                        );
                    }
                    if self.msrv.meets(cx, msrvs::ITER_FLATTEN) {
                        // use the sourcemap to get the span of the closure
                        iter_filter::check(cx, expr, arg, span);
                    }
                },
                (sym::find, [arg]) => {
                    if let Some((sym::cloned, recv2, [], _span2, _)) = method_call(recv) {
                        // if `arg` has side-effect, the semantic will change
                        iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::FixClosure(name, arg),
                            false,
                        );
                    }
                },
                (sym::filter_map, [arg]) => {
                    unnecessary_filter_map::check(cx, expr, arg, name);
                    filter_map_bool_then::check(cx, expr, arg, call_span);
                    filter_map_identity::check(cx, expr, arg, span);
                },
                (sym::find_map, [arg]) => {
                    unnecessary_filter_map::check(cx, expr, arg, name);
                },
                (sym::flat_map, [arg]) => {
                    flat_map_identity::check(cx, expr, arg, span);
                    flat_map_option::check(cx, expr, arg, span);
                },
                (sym::flatten, []) => match method_call(recv) {
                    Some((sym::map, recv, [map_arg], map_span, _)) => {
                        map_flatten::check(cx, expr, recv, map_arg, map_span);
                    },
                    Some((sym::cloned, recv2, [], _, _)) => iter_overeager_cloned::check(
                        cx,
                        expr,
                        recv,
                        recv2,
                        iter_overeager_cloned::Op::LaterCloned,
                        true,
                    ),
                    _ => {},
                },
                (sym::fold, [init, acc]) => {
                    manual_try_fold::check(cx, expr, init, acc, call_span, self.msrv);
                    unnecessary_fold::check(cx, expr, init, acc, span);
                },
                (sym::for_each, [arg]) => match method_call(recv) {
                    Some((sym::inspect, _, [_], span2, _)) => inspect_for_each::check(cx, expr, span2),
                    Some((sym::cloned, recv2, [], _, _)) => iter_overeager_cloned::check(
                        cx,
                        expr,
                        recv,
                        recv2,
                        iter_overeager_cloned::Op::NeedlessMove(arg),
                        false,
                    ),
                    _ => {},
                },
                (sym::get, [arg]) => {
                    get_first::check(cx, expr, recv, arg);
                    get_last_with_len::check(cx, expr, recv, arg);
                },
                (sym::get_or_insert_with, [arg]) => {
                    unnecessary_lazy_eval::check(cx, expr, recv, arg, "get_or_insert");
                },
                (sym::hash, [arg]) => {
                    unit_hash::check(cx, expr, recv, arg);
                },
                (sym::is_empty, []) => {
                    match method_call(recv) {
                        Some((prev_method @ (sym::as_bytes | sym::bytes), prev_recv, [], _, _)) => {
                            needless_as_bytes::check(cx, prev_method, name, prev_recv, expr.span);
                        },
                        Some((sym::as_str, recv, [], as_str_span, _)) => {
                            redundant_as_str::check(cx, expr, recv, as_str_span, span);
                        },
                        _ => {},
                    }
                    is_empty::check(cx, expr, recv);
                },
                (sym::is_file, []) => filetype_is_file::check(cx, expr, recv),
                (sym::is_digit, [radix]) => is_digit_ascii_radix::check(cx, expr, recv, radix, self.msrv),
                (sym::is_none, []) => check_is_some_is_none(cx, expr, recv, call_span, false),
                (sym::is_some, []) => check_is_some_is_none(cx, expr, recv, call_span, true),
                (sym::iter | sym::iter_mut | sym::into_iter, []) => {
                    iter_on_single_or_empty_collections::check(cx, expr, name, recv);
                },
                (sym::join, [join_arg]) => {
                    if let Some((sym::collect, _, _, span, _)) = method_call(recv) {
                        unnecessary_join::check(cx, expr, recv, join_arg, span);
                    } else {
                        join_absolute_paths::check(cx, recv, join_arg, expr.span);
                    }
                },
                (sym::last, []) => {
                    if let Some((sym::cloned, recv2, [], _span2, _)) = method_call(recv) {
                        iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::LaterCloned,
                            false,
                        );
                    }
                    double_ended_iterator_last::check(cx, expr, recv, call_span);
                },
                (sym::len, []) => {
                    if let Some((prev_method @ (sym::as_bytes | sym::bytes), prev_recv, [], _, _)) = method_call(recv) {
                        needless_as_bytes::check(cx, prev_method, sym::len, prev_recv, expr.span);
                    }
                },
                (sym::lock, []) => {
                    mut_mutex_lock::check(cx, expr, recv, span);
                },
                (name @ (sym::map | sym::map_err), [m_arg]) => {
                    if name == sym::map {
                        map_clone::check(cx, expr, recv, m_arg, self.msrv);
                        map_with_unused_argument_over_ranges::check(cx, expr, recv, m_arg, self.msrv, span);
                        manual_is_variant_and::check_map(cx, expr);
                        match method_call(recv) {
                            Some((map_name @ (sym::iter | sym::into_iter), recv2, _, _, _)) => {
                                iter_kv_map::check(cx, map_name, expr, recv2, m_arg, self.msrv);
                            },
                            Some((sym::cloned, recv2, [], _, _)) => iter_overeager_cloned::check(
                                cx,
                                expr,
                                recv,
                                recv2,
                                iter_overeager_cloned::Op::NeedlessMove(m_arg),
                                false,
                            ),
                            _ => {},
                        }
                    } else {
                        map_err_ignore::check(cx, expr, m_arg);
                    }
                    if let Some((name, recv2, args, span2, _)) = method_call(recv) {
                        match (name, args) {
                            (sym::as_mut, []) => option_as_ref_deref::check(cx, expr, recv2, m_arg, true, self.msrv),
                            (sym::as_ref, []) => option_as_ref_deref::check(cx, expr, recv2, m_arg, false, self.msrv),
                            (sym::filter, [f_arg]) => {
                                filter_map::check(cx, expr, recv2, f_arg, span2, recv, m_arg, span, false);
                            },
                            (sym::find, [f_arg]) => {
                                filter_map::check(cx, expr, recv2, f_arg, span2, recv, m_arg, span, true);
                            },
                            _ => {},
                        }
                    }
                    map_identity::check(cx, expr, recv, m_arg, name, span);
                    manual_inspect::check(cx, expr, m_arg, name, span, self.msrv);
                },
                (sym::map_or, [def, map]) => {
                    option_map_or_none::check(cx, expr, recv, def, map);
                    manual_ok_or::check(cx, expr, recv, def, map);
                    unnecessary_map_or::check(cx, expr, recv, def, map, span, self.msrv);
                },
                (sym::map_or_else, [def, map]) => {
                    result_map_or_else_none::check(cx, expr, recv, def, map);
                    unnecessary_result_map_or_else::check(cx, expr, recv, def, map);
                },
                (sym::next, []) => {
                    if let Some((name2, recv2, args2, _, _)) = method_call(recv) {
                        match (name2, args2) {
                            (sym::cloned, []) => iter_overeager_cloned::check(
                                cx,
                                expr,
                                recv,
                                recv2,
                                iter_overeager_cloned::Op::LaterCloned,
                                false,
                            ),
                            (sym::filter, [arg]) => filter_next::check(cx, expr, recv2, arg),
                            (sym::filter_map, [arg]) => filter_map_next::check(cx, expr, recv2, arg, self.msrv),
                            (sym::iter, []) => iter_next_slice::check(cx, expr, recv2),
                            (sym::skip, [arg]) => iter_skip_next::check(cx, expr, recv2, arg),
                            (sym::skip_while, [_]) => skip_while_next::check(cx, expr),
                            (sym::rev, []) => manual_next_back::check(cx, expr, recv, recv2),
                            _ => {},
                        }
                    }
                },
                (sym::nth, [n_arg]) => match method_call(recv) {
                    Some((sym::bytes, recv2, [], _, _)) => bytes_nth::check(cx, expr, recv2, n_arg),
                    Some((sym::cloned, recv2, [], _, _)) => iter_overeager_cloned::check(
                        cx,
                        expr,
                        recv,
                        recv2,
                        iter_overeager_cloned::Op::LaterCloned,
                        false,
                    ),
                    Some((iter_method @ (sym::iter | sym::iter_mut), iter_recv, [], iter_span, _)) => {
                        if !iter_nth::check(cx, expr, iter_recv, iter_method, iter_span, span) {
                            iter_nth_zero::check(cx, expr, recv, n_arg);
                        }
                    },
                    _ => iter_nth_zero::check(cx, expr, recv, n_arg),
                },
                (sym::ok_or_else, [arg]) => {
                    unnecessary_lazy_eval::check(cx, expr, recv, arg, "ok_or");
                },
                (sym::open, [_]) => {
                    open_options::check(cx, expr, recv);
                },
                (sym::or_else, [arg]) => {
                    if !bind_instead_of_map::check_or_else_err(cx, expr, recv, arg) {
                        unnecessary_lazy_eval::check(cx, expr, recv, arg, "or");
                    }
                },
                (sym::push, [arg]) => {
                    path_buf_push_overwrite::check(cx, expr, arg);
                },
                (sym::read_to_end, [_]) => {
                    verbose_file_reads::check(cx, expr, recv, verbose_file_reads::READ_TO_END_MSG);
                },
                (sym::read_to_string, [_]) => {
                    verbose_file_reads::check(cx, expr, recv, verbose_file_reads::READ_TO_STRING_MSG);
                },
                (sym::read_line, [arg]) => {
                    read_line_without_trim::check(cx, expr, recv, arg);
                },
                (sym::repeat, [arg]) => {
                    repeat_once::check(cx, expr, recv, arg);
                },
                (name @ (sym::replace | sym::replacen), [arg1, arg2] | [arg1, arg2, _]) => {
                    no_effect_replace::check(cx, expr, arg1, arg2);

                    // Check for repeated `str::replace` calls to perform `collapsible_str_replace` lint
                    if self.msrv.meets(cx, msrvs::PATTERN_TRAIT_CHAR_ARRAY)
                        && name == sym::replace
                        && let Some((sym::replace, ..)) = method_call(recv)
                    {
                        collapsible_str_replace::check(cx, expr, arg1, arg2);
                    }
                },
                (sym::resize, [count_arg, default_arg]) => {
                    vec_resize_to_zero::check(cx, expr, count_arg, default_arg, span);
                },
                (sym::seek, [arg]) => {
                    if self.msrv.meets(cx, msrvs::SEEK_FROM_CURRENT) {
                        seek_from_current::check(cx, expr, recv, arg);
                    }
                    if self.msrv.meets(cx, msrvs::SEEK_REWIND) {
                        seek_to_start_instead_of_rewind::check(cx, expr, recv, arg, span);
                    }
                },
                (sym::skip, [arg]) => {
                    iter_skip_zero::check(cx, expr, arg);
                    iter_out_of_bounds::check_skip(cx, expr, recv, arg);

                    if let Some((sym::cloned, recv2, [], _span2, _)) = method_call(recv) {
                        iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::LaterCloned,
                            false,
                        );
                    }
                },
                (sym::sort, []) => {
                    stable_sort_primitive::check(cx, expr, recv);
                },
                (sym::sort_by, [arg]) => {
                    unnecessary_sort_by::check(cx, expr, recv, arg, false);
                },
                (sym::sort_unstable_by, [arg]) => {
                    unnecessary_sort_by::check(cx, expr, recv, arg, true);
                },
                (sym::split, [arg]) => {
                    str_split::check(cx, expr, recv, arg);
                },
                (sym::splitn | sym::rsplitn, [count_arg, pat_arg]) => {
                    if let Some(Constant::Int(count)) = ConstEvalCtxt::new(cx).eval(count_arg) {
                        suspicious_splitn::check(cx, name, expr, recv, count);
                        str_splitn::check(cx, name, expr, recv, pat_arg, count, self.msrv);
                    }
                },
                (sym::splitn_mut | sym::rsplitn_mut, [count_arg, _]) => {
                    if let Some(Constant::Int(count)) = ConstEvalCtxt::new(cx).eval(count_arg) {
                        suspicious_splitn::check(cx, name, expr, recv, count);
                    }
                },
                (sym::step_by, [arg]) => iterator_step_by_zero::check(cx, expr, arg),
                (sym::take, [arg]) => {
                    iter_out_of_bounds::check_take(cx, expr, recv, arg);
                    manual_repeat_n::check(cx, expr, recv, arg, self.msrv);
                    if let Some((sym::cloned, recv2, [], _span2, _)) = method_call(recv) {
                        iter_overeager_cloned::check(
                            cx,
                            expr,
                            recv,
                            recv2,
                            iter_overeager_cloned::Op::LaterCloned,
                            false,
                        );
                    }
                },
                (sym::take, []) => needless_option_take::check(cx, expr, recv),
                (sym::then, [arg]) => {
                    if !self.msrv.meets(cx, msrvs::BOOL_THEN_SOME) {
                        return;
                    }
                    unnecessary_lazy_eval::check(cx, expr, recv, arg, "then_some");
                },
                (sym::try_into, []) if is_trait_method(cx, expr, sym::TryInto) => {
                    unnecessary_fallible_conversions::check_method(cx, expr);
                },
                (sym::to_owned, []) => {
                    if !suspicious_to_owned::check(cx, expr, recv) {
                        implicit_clone::check(cx, name, expr, recv);
                    }
                },
                (sym::to_os_string | sym::to_path_buf | sym::to_vec, []) => {
                    implicit_clone::check(cx, name, expr, recv);
                },
                (sym::type_id, []) => {
                    type_id_on_box::check(cx, recv, expr.span);
                },
                (sym::unwrap, []) => {
                    match method_call(recv) {
                        Some((sym::get, recv, [get_arg], _, _)) => {
                            get_unwrap::check(cx, expr, recv, get_arg, false);
                        },
                        Some((sym::get_mut, recv, [get_arg], _, _)) => {
                            get_unwrap::check(cx, expr, recv, get_arg, true);
                        },
                        Some((sym::or, recv, [or_arg], or_span, _)) => {
                            or_then_unwrap::check(cx, expr, recv, or_arg, or_span);
                        },
                        _ => {},
                    }
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::unwrap_or, [u_arg]) => {
                    match method_call(recv) {
                        Some((arith @ (sym::checked_add | sym::checked_sub | sym::checked_mul), lhs, [rhs], _, _)) => {
                            manual_saturating_arithmetic::check(
                                cx,
                                expr,
                                lhs,
                                rhs,
                                u_arg,
                                &arith.as_str()[const { "checked_".len() }..],
                            );
                        },
                        Some((sym::map, m_recv, [m_arg], span, _)) => {
                            option_map_unwrap_or::check(cx, expr, m_recv, m_arg, recv, u_arg, span, self.msrv);
                        },
                        Some((then_method @ (sym::then | sym::then_some), t_recv, [t_arg], _, _)) => {
                            obfuscated_if_else::check(cx, expr, t_recv, t_arg, Some(u_arg), then_method, name);
                        },
                        _ => {},
                    }
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::unwrap_or_default, []) => {
                    match method_call(recv) {
                        Some((sym::map, m_recv, [arg], span, _)) => {
                            manual_is_variant_and::check(cx, expr, m_recv, arg, span, self.msrv);
                        },
                        Some((then_method @ (sym::then | sym::then_some), t_recv, [t_arg], _, _)) => {
                            obfuscated_if_else::check(
                                cx,
                                expr,
                                t_recv,
                                t_arg,
                                None,
                                then_method,
                                sym::unwrap_or_default,
                            );
                        },
                        _ => {},
                    }
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::unwrap_or_else, [u_arg]) => {
                    match method_call(recv) {
                        Some((sym::map, recv, [map_arg], _, _))
                            if map_unwrap_or::check(cx, expr, recv, map_arg, u_arg, self.msrv) => {},
                        Some((then_method @ (sym::then | sym::then_some), t_recv, [t_arg], _, _)) => {
                            obfuscated_if_else::check(
                                cx,
                                expr,
                                t_recv,
                                t_arg,
                                Some(u_arg),
                                then_method,
                                sym::unwrap_or_else,
                            );
                        },
                        _ => {
                            unnecessary_lazy_eval::check(cx, expr, recv, u_arg, "unwrap_or");
                        },
                    }
                    unnecessary_literal_unwrap::check(cx, expr, recv, name, args);
                },
                (sym::wake, []) => {
                    waker_clone_wake::check(cx, expr, recv);
                },
                (sym::write, []) => {
                    readonly_write_lock::check(cx, expr, recv);
                },
                (sym::zip, [arg]) => {
                    if let ExprKind::MethodCall(name, iter_recv, [], _) = recv.kind
                        && name.ident.name == sym::iter
                    {
                        range_zip_with_len::check(cx, expr, iter_recv, arg);
                    }
                },
                _ => {},
            }
        }
        // Handle method calls whose receiver and arguments may come from expansion
        if let ExprKind::MethodCall(path, recv, args, _call_span) = expr.kind {
            match (path.ident.name, args) {
                (sym::expect, [_]) if !matches!(method_call(recv), Some((sym::ok | sym::err, _, [], _, _))) => {
                    unwrap_expect_used::check(
                        cx,
                        expr,
                        recv,
                        false,
                        self.allow_expect_in_consts,
                        self.allow_expect_in_tests,
                        unwrap_expect_used::Variant::Expect,
                    );
                },
                (sym::expect_err, [_]) => {
                    unwrap_expect_used::check(
                        cx,
                        expr,
                        recv,
                        true,
                        self.allow_expect_in_consts,
                        self.allow_expect_in_tests,
                        unwrap_expect_used::Variant::Expect,
                    );
                },
                (sym::unwrap, []) => {
                    unwrap_expect_used::check(
                        cx,
                        expr,
                        recv,
                        false,
                        self.allow_unwrap_in_consts,
                        self.allow_unwrap_in_tests,
                        unwrap_expect_used::Variant::Unwrap,
                    );
                },
                (sym::unwrap_err, []) => {
                    unwrap_expect_used::check(
                        cx,
                        expr,
                        recv,
                        true,
                        self.allow_unwrap_in_consts,
                        self.allow_unwrap_in_tests,
                        unwrap_expect_used::Variant::Unwrap,
                    );
                },
                _ => {},
            }
        }
    }
}

fn check_is_some_is_none(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, call_span: Span, is_some: bool) {
    match method_call(recv) {
        Some((name @ (sym::find | sym::position | sym::rposition), f_recv, [arg], span, _)) => {
            search_is_some::check(cx, expr, name, is_some, f_recv, arg, recv, span);
        },
        Some((sym::get, f_recv, [arg], _, _)) => {
            unnecessary_get_then_check::check(cx, call_span, recv, f_recv, arg, is_some);
        },
        Some((sym::first, f_recv, [], _, _)) => {
            unnecessary_first_then_check::check(cx, call_span, recv, f_recv, is_some);
        },
        _ => {},
    }
}

/// Used for `lint_binary_expr_with_method_call`.
#[derive(Copy, Clone)]
struct BinaryExprInfo<'a> {
    expr: &'a Expr<'a>,
    chain: &'a Expr<'a>,
    other: &'a Expr<'a>,
    eq: bool,
}

/// Checks for the `CHARS_NEXT_CMP` and `CHARS_LAST_CMP` lints.
fn lint_binary_expr_with_method_call(cx: &LateContext<'_>, info: &mut BinaryExprInfo<'_>) {
    macro_rules! lint_with_both_lhs_and_rhs {
        ($func:expr, $cx:expr, $info:ident) => {
            if !$func($cx, $info) {
                ::std::mem::swap(&mut $info.chain, &mut $info.other);
                if $func($cx, $info) {
                    return;
                }
            }
        };
    }

    lint_with_both_lhs_and_rhs!(chars_cmp::check_next, cx, info);
    lint_with_both_lhs_and_rhs!(chars_cmp::check_last, cx, info);
    lint_with_both_lhs_and_rhs!(chars_cmp::check_next_unwrap, cx, info);
    lint_with_both_lhs_and_rhs!(chars_cmp::check_last_unwrap, cx, info);
}

const FN_HEADER: hir::FnHeader = hir::FnHeader {
    safety: hir::HeaderSafety::Normal(hir::Safety::Safe),
    constness: hir::Constness::NotConst,
    asyncness: hir::IsAsync::NotAsync,
    abi: ExternAbi::Rust,
};

struct ShouldImplTraitCase {
    trait_name: &'static str,
    method_name: Symbol,
    param_count: usize,
    fn_header: hir::FnHeader,
    // implicit self kind expected (none, self, &self, ...)
    self_kind: SelfKind,
    // checks against the output type
    output_type: OutType,
    // certain methods with explicit lifetimes can't implement the equivalent trait method
    lint_explicit_lifetime: bool,
}
impl ShouldImplTraitCase {
    const fn new(
        trait_name: &'static str,
        method_name: Symbol,
        param_count: usize,
        fn_header: hir::FnHeader,
        self_kind: SelfKind,
        output_type: OutType,
        lint_explicit_lifetime: bool,
    ) -> ShouldImplTraitCase {
        ShouldImplTraitCase {
            trait_name,
            method_name,
            param_count,
            fn_header,
            self_kind,
            output_type,
            lint_explicit_lifetime,
        }
    }

    fn lifetime_param_cond(&self, impl_item: &hir::ImplItem<'_>) -> bool {
        self.lint_explicit_lifetime
            || !impl_item.generics.params.iter().any(|p| {
                matches!(
                    p.kind,
                    hir::GenericParamKind::Lifetime {
                        kind: hir::LifetimeParamKind::Explicit
                    }
                )
            })
    }
}

#[rustfmt::skip]
const TRAIT_METHODS: [ShouldImplTraitCase; 30] = [
    ShouldImplTraitCase::new("std::ops::Add", sym::add,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::convert::AsMut", sym::as_mut,  1,  FN_HEADER,  SelfKind::RefMut,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::convert::AsRef", sym::as_ref,  1,  FN_HEADER,  SelfKind::Ref,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::ops::BitAnd", sym::bitand,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::BitOr", sym::bitor,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::BitXor", sym::bitxor,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::borrow::Borrow", sym::borrow,  1,  FN_HEADER,  SelfKind::Ref,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::borrow::BorrowMut", sym::borrow_mut,  1,  FN_HEADER,  SelfKind::RefMut,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::clone::Clone", sym::clone,  1,  FN_HEADER,  SelfKind::Ref,  OutType::Any, true),
    ShouldImplTraitCase::new("std::cmp::Ord", sym::cmp,  2,  FN_HEADER,  SelfKind::Ref,  OutType::Any, true),
    ShouldImplTraitCase::new("std::default::Default", kw::Default,  0,  FN_HEADER,  SelfKind::No,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Deref", sym::deref,  1,  FN_HEADER,  SelfKind::Ref,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::ops::DerefMut", sym::deref_mut,  1,  FN_HEADER,  SelfKind::RefMut,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::ops::Div", sym::div,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Drop", sym::drop,  1,  FN_HEADER,  SelfKind::RefMut,  OutType::Unit, true),
    ShouldImplTraitCase::new("std::cmp::PartialEq", sym::eq,  2,  FN_HEADER,  SelfKind::Ref,  OutType::Bool, true),
    ShouldImplTraitCase::new("std::iter::FromIterator", sym::from_iter,  1,  FN_HEADER,  SelfKind::No,  OutType::Any, true),
    ShouldImplTraitCase::new("std::str::FromStr", sym::from_str,  1,  FN_HEADER,  SelfKind::No,  OutType::Any, true),
    ShouldImplTraitCase::new("std::hash::Hash", sym::hash,  2,  FN_HEADER,  SelfKind::Ref,  OutType::Unit, true),
    ShouldImplTraitCase::new("std::ops::Index", sym::index,  2,  FN_HEADER,  SelfKind::Ref,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::ops::IndexMut", sym::index_mut,  2,  FN_HEADER,  SelfKind::RefMut,  OutType::Ref, true),
    ShouldImplTraitCase::new("std::iter::IntoIterator", sym::into_iter,  1,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Mul", sym::mul,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Neg", sym::neg,  1,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::iter::Iterator", sym::next,  1,  FN_HEADER,  SelfKind::RefMut,  OutType::Any, false),
    ShouldImplTraitCase::new("std::ops::Not", sym::not,  1,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Rem", sym::rem,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Shl", sym::shl,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Shr", sym::shr,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
    ShouldImplTraitCase::new("std::ops::Sub", sym::sub,  2,  FN_HEADER,  SelfKind::Value,  OutType::Any, true),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SelfKind {
    Value,
    Ref,
    RefMut,
    No, // When we want the first argument type to be different than `Self`
}

impl SelfKind {
    fn matches<'a>(self, cx: &LateContext<'a>, parent_ty: Ty<'a>, ty: Ty<'a>) -> bool {
        fn matches_value<'a>(cx: &LateContext<'a>, parent_ty: Ty<'a>, ty: Ty<'a>) -> bool {
            if ty == parent_ty {
                true
            } else if let Some(boxed_ty) = ty.boxed_ty() {
                boxed_ty == parent_ty
            } else if is_type_diagnostic_item(cx, ty, sym::Rc) || is_type_diagnostic_item(cx, ty, sym::Arc) {
                if let ty::Adt(_, args) = ty.kind() {
                    args.types().next() == Some(parent_ty)
                } else {
                    false
                }
            } else {
                false
            }
        }

        fn matches_ref<'a>(cx: &LateContext<'a>, mutability: hir::Mutability, parent_ty: Ty<'a>, ty: Ty<'a>) -> bool {
            if let ty::Ref(_, t, m) = *ty.kind() {
                return m == mutability && t == parent_ty;
            }

            let trait_sym = match mutability {
                hir::Mutability::Not => sym::AsRef,
                hir::Mutability::Mut => sym::AsMut,
            };

            let Some(trait_def_id) = cx.tcx.get_diagnostic_item(trait_sym) else {
                return false;
            };
            implements_trait(cx, ty, trait_def_id, &[parent_ty.into()])
        }

        fn matches_none<'a>(cx: &LateContext<'a>, parent_ty: Ty<'a>, ty: Ty<'a>) -> bool {
            !matches_value(cx, parent_ty, ty)
                && !matches_ref(cx, hir::Mutability::Not, parent_ty, ty)
                && !matches_ref(cx, hir::Mutability::Mut, parent_ty, ty)
        }

        match self {
            Self::Value => matches_value(cx, parent_ty, ty),
            Self::Ref => matches_ref(cx, hir::Mutability::Not, parent_ty, ty) || ty == parent_ty && is_copy(cx, ty),
            Self::RefMut => matches_ref(cx, hir::Mutability::Mut, parent_ty, ty),
            Self::No => matches_none(cx, parent_ty, ty),
        }
    }

    #[must_use]
    fn description(self) -> &'static str {
        match self {
            Self::Value => "`self` by value",
            Self::Ref => "`self` by reference",
            Self::RefMut => "`self` by mutable reference",
            Self::No => "no `self`",
        }
    }
}

#[derive(Clone, Copy)]
enum OutType {
    Unit,
    Bool,
    Any,
    Ref,
}

impl OutType {
    fn matches(self, ty: &hir::FnRetTy<'_>) -> bool {
        let is_unit = |ty: &hir::Ty<'_>| matches!(ty.kind, hir::TyKind::Tup(&[]));
        match (self, ty) {
            (Self::Unit, &hir::FnRetTy::DefaultReturn(_)) => true,
            (Self::Unit, &hir::FnRetTy::Return(ty)) if is_unit(ty) => true,
            (Self::Bool, &hir::FnRetTy::Return(ty)) if is_bool(ty) => true,
            (Self::Any, &hir::FnRetTy::Return(ty)) if !is_unit(ty) => true,
            (Self::Ref, &hir::FnRetTy::Return(ty)) => matches!(ty.kind, hir::TyKind::Ref(_, _)),
            _ => false,
        }
    }
}

fn fn_header_equals(expected: hir::FnHeader, actual: hir::FnHeader) -> bool {
    expected.constness == actual.constness && expected.safety == actual.safety && expected.asyncness == actual.asyncness
}

pub fn register_lint_passes(store: &mut rustc_lint::LintStore, conf: &'static Conf, format_args: FormatArgsStorage) {
    store.register_late_pass(move |_| Box::new(Methods::new(conf, format_args.clone())));
}
