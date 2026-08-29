#![feature(control_flow_into_value)]
#![feature(deref_patterns)]
#![feature(exact_div)]
#![feature(f128)]
#![feature(f16)]
#![feature(iter_intersperse)]
#![feature(iter_partition_in_place)]
#![feature(macro_metavar_expr)]
#![feature(macro_metavar_expr_concat)]
#![feature(never_type)]
#![feature(rustc_private)]
#![feature(stmt_expr_attributes)]
#![feature(unwrap_infallible)]
#![recursion_limit = "512"]
#![expect(clippy::literal_string_with_formatting_args, clippy::must_use_candidate)]
#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_qualifications,
    rustc::internal
)]

extern crate rustc_abi;
extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_pretty;
extern crate rustc_hir_typeck;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_lexer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_parse_format;
extern crate rustc_resolve;
extern crate rustc_span;
extern crate rustc_structures;
extern crate rustc_target;
extern crate rustc_trait_selection;

#[macro_use]
extern crate clippy_utils;

#[macro_use]
extern crate declare_clippy_lint;

pub mod absolute_paths;
pub mod almost_complete_range;
pub mod approx_const;
pub mod arbitrary_source_item_ordering;
pub mod arc_with_non_send_sync;
pub mod as_conversions;
pub mod asm_syntax;
pub mod assert_is_empty;
pub mod assertions_on_constants;
pub mod assertions_on_result_states;
pub mod assigning_clones;
pub mod async_yields_async;
pub mod attrs;
pub mod await_holding_invalid;
pub mod bit_width;
pub mod block_scrutinee;
pub mod blocks_in_conditions;
pub mod bool_assert_comparison;
pub mod bool_comparison;
pub mod bool_to_int_with_if;
pub mod booleans;
pub mod borrow_deref_ref;
pub mod box_default;
pub mod byte_char_slices;
pub mod cargo;
pub mod casts;
pub mod cfg_not_test;
pub mod checked_conversions;
pub mod cloned_ref_to_slice_refs;
pub mod coerce_container_to_any;
pub mod cognitive_complexity;
pub mod collapsible_if;
pub mod collection_is_never_read;
pub mod comparison_chain;
pub mod copy_iterator;
pub mod crate_in_macro_def;
pub mod create_dir;
pub mod dbg_macro;
pub mod default;
pub mod default_constructed_unit_structs;
pub mod default_instead_of_iter_empty;
pub mod default_numeric_fallback;
pub mod default_union_representation;
pub mod definition_in_module_root;
pub mod dereference;
pub mod derivable_impls;
pub mod derive;
pub mod disallowed_fields;
pub mod disallowed_macros;
pub mod disallowed_methods;
pub mod disallowed_names;
pub mod disallowed_script_idents;
pub mod disallowed_types;
pub mod doc;
pub mod double_parens;
pub mod drop_forget_ref;
pub mod duplicate_mod;
pub mod duration_suboptimal_units;
pub mod else_if_without_else;
pub mod empty_drop;
pub mod empty_enums;
pub mod empty_line_after;
pub mod empty_with_brackets;
pub mod endian_bytes;
pub mod entry;
pub mod enum_clike;
pub mod equatable_if_let;
pub mod error_impl_error;
pub mod escape;
pub mod eta_reduction;
pub mod excessive_bools;
pub mod excessive_nesting;
pub mod exhaustive_items;
pub mod exit;
pub mod explicit_write;
pub mod extra_unused_type_parameters;
pub mod fallible_impl_from;
pub mod field_scoped_visibility_modifiers;
pub mod float_literal;
pub mod floating_point_arithmetic;
pub mod format;
pub mod format_args;
pub mod format_impl;
pub mod format_push_string;
pub mod formatting;
pub mod four_forward_slashes;
pub mod from_over_into;
pub mod from_raw_with_void_ptr;
pub mod from_str_radix_10;
pub mod functions;
pub mod future_not_send;
pub mod if_let_mutex;
pub mod if_not_else;
pub mod if_then_some_else_none;
pub mod ifs;
pub mod ignored_unit_patterns;
pub mod impl_hash_with_borrow_str_and_bytes;
pub mod implicit_hasher;
pub mod implicit_return;
pub mod implicit_saturating_add;
pub mod implicit_saturating_sub;
pub mod implied_bounds_in_impls;
pub mod incompatible_msrv;
pub mod inconsistent_struct_constructor;
pub mod index_refutable_slice;
pub mod indexing_slicing;
pub mod ineffective_open_options;
pub mod infallible_try_from;
pub mod infinite_iter;
pub mod inherent_impl;
pub mod inherent_to_string;
pub mod init_numbered_fields;
pub mod inline_fn_without_body;
pub mod inline_trait_bounds;
pub mod int_plus_one;
pub mod item_name_repetitions;
pub mod items_after_statements;
pub mod items_after_test_module;
pub mod iter_not_returning_iterator;
pub mod iter_over_hash_type;
pub mod iter_without_into_iter;
pub mod large_const_arrays;
pub mod large_enum_variant;
pub mod large_futures;
pub mod large_include_file;
pub mod large_stack_arrays;
pub mod large_stack_frames;
pub mod legacy_numeric_constants;
pub mod len_without_is_empty;
pub mod len_zero;
pub mod let_if_seq;
pub mod let_underscore;
pub mod let_with_type_underscore;
pub mod lifetimes;
pub mod literal_representation;
pub mod literal_string_with_formatting_args;
pub mod loops;
pub mod macro_metavars_in_unsafe;
pub mod macro_use;
pub mod main_recursion;
pub mod manual_abs_diff;
pub mod manual_assert;
pub mod manual_assert_eq;
pub mod manual_async_fn;
pub mod manual_bits;
pub mod manual_checked_ops;
pub mod manual_clamp;
pub mod manual_float_methods;
pub mod manual_hash_one;
pub mod manual_ignore_case_cmp;
pub mod manual_ilog2;
pub mod manual_is_ascii_check;
pub mod manual_is_power_of_two;
pub mod manual_let_else;
pub mod manual_main_separator_str;
pub mod manual_non_exhaustive;
pub mod manual_noop_waker;
pub mod manual_option_as_slice;
pub mod manual_pop_if;
pub mod manual_range_patterns;
pub mod manual_rem_euclid;
pub mod manual_retain;
pub mod manual_rotate;
pub mod manual_slice_size_calculation;
pub mod manual_string_new;
pub mod manual_strip;
pub mod manual_take;
pub mod map_unit_fn;
pub mod match_result_ok;
pub mod matches;
pub mod mem_replace;
pub mod methods;
pub mod min_ident_chars;
pub mod minmax;
pub mod misc;
pub mod misc_early;
pub mod mismatching_type_param_order;
pub mod missing_assert_message;
pub mod missing_asserts_for_indexing;
pub mod missing_const_for_fn;
pub mod missing_const_for_thread_local;
pub mod missing_doc;
pub mod missing_enforced_import_rename;
pub mod missing_fields_in_debug;
pub mod missing_inline;
pub mod missing_trait_methods;
pub mod mixed_read_write_in_expression;
pub mod module_style;
pub mod multi_assignments;
pub mod multiple_bound_locations;
pub mod multiple_unsafe_ops_per_block;
pub mod mut_key;
pub mod mut_mut;
pub mod mutable_debug_assertion;
pub mod mutex_atomic;
pub mod needless_arbitrary_self_type;
pub mod needless_bool;
pub mod needless_borrowed_ref;
pub mod needless_borrows_for_generic_args;
pub mod needless_continue;
pub mod needless_else;
pub mod needless_for_each;
pub mod needless_ifs;
pub mod needless_late_init;
pub mod needless_maybe_sized;
pub mod needless_nonzero_get;
pub mod needless_parens_on_range_literals;
pub mod needless_pass_by_ref_mut;
pub mod needless_pass_by_value;
pub mod needless_question_mark;
pub mod needless_update;
pub mod neg_cmp_op_on_partial_ord;
pub mod neg_multiply;
pub mod new_without_default;
pub mod no_effect;
pub mod no_mangle_with_rust_abi;
pub mod non_canonical_impls;
pub mod non_copy_const;
pub mod non_expressive_names;
pub mod non_octal_unix_permissions;
pub mod non_send_fields_in_send_ty;
pub mod non_std_lazy_statics;
pub mod non_zero_suggestions;
pub mod nonnull_unchecked_on_box_ptr;
pub mod nonstandard_macro_braces;
pub mod octal_escapes;
pub mod only_used_in_recursion;
pub mod operators;
pub mod option_env_unwrap;
pub mod option_if_let_else;
pub mod panic_in_result_fn;
pub mod panic_unimplemented;
pub mod panicking_overflow_checks;
pub mod partial_pub_fields;
pub mod partialeq_ne_impl;
pub mod partialeq_to_none;
pub mod pass_by_ref_or_value;
pub mod pathbuf_init_then_push;
pub mod pattern_type_mismatch;
pub mod permissions_set_readonly_false;
pub mod pointers_in_nomem_asm_block;
pub mod precedence;
pub mod ptr;
pub mod pub_underscore_fields;
pub mod pub_use;
pub mod question_mark;
pub mod question_mark_used;
pub mod ranges;
pub mod raw_strings;
pub mod rc_clone_in_vec_init;
pub mod read_zero_byte_vec;
pub mod redundant_async_block;
pub mod redundant_clone;
pub mod redundant_closure_call;
pub mod redundant_else;
pub mod redundant_field_names;
pub mod redundant_locals;
pub mod redundant_pub_crate;
pub mod redundant_slicing;
pub mod redundant_static_lifetimes;
pub mod redundant_test_prefix;
pub mod redundant_type_annotations;
pub mod ref_option_ref;
pub mod ref_patterns;
pub mod reference;
pub mod regex;
pub mod repeat_vec_with_capacity;
pub mod replace_box;
pub mod reserve_after_initialization;
pub mod rest_when_destructuring_struct;
pub mod return_self_not_must_use;
pub mod returns;
pub mod same_length_and_capacity;
pub mod same_name_method;
pub mod self_named_constructors;
pub mod semicolon_block;
pub mod semicolon_if_nothing_returned;
pub mod serde_api;
pub mod set_contains_or_insert;
pub mod shadow;
pub mod significant_drop_tightening;
pub mod single_call_fn;
pub mod single_char_lifetime_names;
pub mod single_component_path_imports;
pub mod single_option_map;
pub mod single_range_in_vec_init;
pub mod size_of_in_element_count;
pub mod size_of_ref;
pub mod slow_vector_initialization;
pub mod std_instead_of_core;
pub mod string_patterns;
pub mod strings;
pub mod strlen_on_c_strings;
pub mod suspicious_operation_groupings;
pub mod suspicious_trait_impl;
pub mod suspicious_xor_used_as_pow;
pub mod swap;
pub mod swap_ptr_to_ref;
pub mod tabs_in_doc_comments;
pub mod temporary_assignment;
pub mod tests_outside_test_module;
pub mod time_subtraction;
pub mod to_digit_is_some;
pub mod to_string_trait_impl;
pub mod toplevel_ref_arg;
pub mod trailing_empty_array;
pub mod trait_bounds;
pub mod transmute;
pub mod tuple_array_conversions;
pub mod types;
pub mod unconditional_recursion;
pub mod undocumented_unsafe_blocks;
pub mod unicode;
pub mod uninhabited_references;
pub mod uninit_vec;
pub mod unit_return_expecting_ord;
pub mod unit_types;
pub mod unnecessary_box_returns;
pub mod unnecessary_literal_bound;
pub mod unnecessary_map_on_constructor;
pub mod unnecessary_mut_passed;
pub mod unnecessary_owned_empty_strings;
pub mod unnecessary_self_imports;
pub mod unnecessary_semicolon;
pub mod unnecessary_struct_initialization;
pub mod unnecessary_wraps;
pub mod unneeded_struct_pattern;
pub mod unnested_or_patterns;
pub mod unsafe_removed_from_name;
pub mod unused_async;
pub mod unused_io_amount;
pub mod unused_peekable;
pub mod unused_result_ok;
pub mod unused_rounding;
pub mod unused_self;
pub mod unused_trait_names;
pub mod unused_unit;
pub mod unwrap;
pub mod unwrap_in_result;
pub mod upper_case_acronyms;
pub mod use_self;
pub mod useless_concat;
pub mod useless_conversion;
pub mod useless_vec;
pub mod vec_init_then_push;
pub mod visibility;
pub mod volatile_composites;
pub mod wildcard_imports;
pub mod with_capacity_zero;
pub mod write;
pub mod zero_div_zero;
pub mod zero_repeat_side_effects;
pub mod zero_sized_map_values;
pub mod zombie_processes;

mod combined_early_pass;
mod combined_late_pass;
mod utils;

pub mod declared_lints;
pub mod deprecated_lints;

use clippy_config::{Conf, sanitize_explanation};
use clippy_utils::macros::FormatArgsStorage;
use rustc_data_structures::fx::FxHashSet;
use rustc_lint::is_lint_pass_required;
use rustc_middle::ty::TyCtxt;
use utils::attr_collector::AttrStorage;

pub fn explain(name: &str) -> i32 {
    let target = format!("clippy::{}", name.to_ascii_uppercase());
    if let Some(info) = declared_lints::LINTS.iter().find(|info| info.lint.name == target) {
        println!("{}", sanitize_explanation(info.explanation));
        // Check if the lint has configuration
        let mut mdconf = Conf::get_metadata();
        let name = name.to_ascii_lowercase();
        mdconf.retain(|cconf| cconf.lints.contains(&&*name));
        if !mdconf.is_empty() {
            println!("### Configuration for {}:\n", info.lint.name_lower());
            for conf in mdconf {
                println!("{conf}");
            }
        }
        0
    } else {
        println!("unknown lint: {name}");
        1
    }
}

/// Register all lints and lint groups with the rustc lint store
///
/// Used in `./src/driver.rs`.
pub fn register_lint_passes(store: &mut rustc_lint::LintStore, conf: &'static Conf) {
    for (old_name, new_name) in deprecated_lints::RENAMED {
        store.register_renamed(old_name, new_name);
    }
    for (name, reason) in deprecated_lints::DEPRECATED {
        store.register_removed(name, reason);
    }

    // NOTE: Do not add any more pre-expansion passes. These should be removed eventually.
    // Due to the architecture of the compiler, currently `cfg_attr` attributes on crate
    // level (i.e `#![cfg_attr(...)]`) will still be expanded even when using a pre-expansion pass.
    store.register_pre_expansion_lint_pass(Box::new(move || Box::new(attrs::EarlyAttributes::new(conf))));
    store.register_pre_expansion_lint_pass(Box::new(move || {
        Box::new(nonstandard_macro_braces::MacroBraces::new(conf))
    }));

    let format_args_storage = FormatArgsStorage::default();
    let attr_storage = AttrStorage::default();

    {
        let format_args = format_args_storage.clone();
        let attrs = attr_storage.clone();
        store.register_early_lint_pass(Box::new(move || {
            Box::new(CombinedEarlyLintPass::new(conf, format_args.clone(), attrs.clone()))
        }));
    }

    store.register_late_lint_pass(Box::new(move |tcx: TyCtxt<'_>| {
        let skippable_lints = tcx.skippable_lints(());
        let is_active = |lints: &rustc_lint::LintVec| is_lint_pass_required(skippable_lints, lints);
        Box::new(CombinedLateLintPass::new(
            tcx,
            conf,
            format_args_storage.clone(),
            attr_storage.clone(),
            &is_active,
        ))
    }));
}

// Fold every early pass into one statically-combined struct (see
// `combined_early_pass`); the method list comes from `early_lint_methods!`.
#[rustfmt::skip]
rustc_lint::early_lint_methods!(
    crate::combined_early_lint_pass,
    [CombinedEarlyLintPass, (conf: &'static Conf, format_args: FormatArgsStorage, attrs: AttrStorage), [
        FormatArgsCollector: utils::format_args_collector::FormatArgsCollector = utils::format_args_collector::FormatArgsCollector::new(format_args.clone()),
        AttrCollector: utils::attr_collector::AttrCollector = utils::attr_collector::AttrCollector::new(attrs.clone()),
        PostExpansionEarlyAttributes: attrs::PostExpansionEarlyAttributes = attrs::PostExpansionEarlyAttributes::new(conf),
        UnnecessarySelfImports: unnecessary_self_imports::UnnecessarySelfImports = unnecessary_self_imports::UnnecessarySelfImports,
        RedundantStaticLifetimes: redundant_static_lifetimes::RedundantStaticLifetimes = redundant_static_lifetimes::RedundantStaticLifetimes::new(conf),
        RedundantFieldNames: redundant_field_names::RedundantFieldNames = redundant_field_names::RedundantFieldNames::new(conf),
        UnnestedOrPatterns: unnested_or_patterns::UnnestedOrPatterns = unnested_or_patterns::UnnestedOrPatterns::new(conf),
        EarlyFunctions: functions::EarlyFunctions = functions::EarlyFunctions,
        Documentation: doc::Documentation = doc::Documentation::new(conf),
        SuspiciousOperationGroupings: suspicious_operation_groupings::SuspiciousOperationGroupings = <suspicious_operation_groupings::SuspiciousOperationGroupings>::default(),
        DoubleParens: double_parens::DoubleParens = double_parens::DoubleParens,
        UnsafeNameRemoval: unsafe_removed_from_name::UnsafeNameRemoval = unsafe_removed_from_name::UnsafeNameRemoval,
        ElseIfWithoutElse: else_if_without_else::ElseIfWithoutElse = else_if_without_else::ElseIfWithoutElse,
        IntPlusOne: int_plus_one::IntPlusOne = int_plus_one::IntPlusOne,
        Formatting: formatting::Formatting = formatting::Formatting,
        MiscEarlyLints: misc_early::MiscEarlyLints = misc_early::MiscEarlyLints,
        UnusedUnit: unused_unit::UnusedUnit = unused_unit::UnusedUnit,
        Precedence: precedence::Precedence = precedence::Precedence,
        NeedlessArbitrarySelfType: needless_arbitrary_self_type::NeedlessArbitrarySelfType = needless_arbitrary_self_type::NeedlessArbitrarySelfType,
        LiteralDigitGrouping: literal_representation::LiteralDigitGrouping = literal_representation::LiteralDigitGrouping::new(conf),
        DecimalLiteralRepresentation: literal_representation::DecimalLiteralRepresentation = literal_representation::DecimalLiteralRepresentation::new(conf),
        TabsInDocComments: tabs_in_doc_comments::TabsInDocComments = tabs_in_doc_comments::TabsInDocComments,
        SingleComponentPathImports: single_component_path_imports::SingleComponentPathImports = single_component_path_imports::SingleComponentPathImports::default(),
        OptionEnvUnwrap: option_env_unwrap::OptionEnvUnwrap = option_env_unwrap::OptionEnvUnwrap,
        NonExpressiveNames: non_expressive_names::NonExpressiveNames = non_expressive_names::NonExpressiveNames::new(conf),
        MacroBraces: nonstandard_macro_braces::MacroBraces = nonstandard_macro_braces::MacroBraces::new(conf),
        InlineAsmX86AttSyntax: asm_syntax::InlineAsmX86AttSyntax = asm_syntax::InlineAsmX86AttSyntax,
        InlineAsmX86IntelSyntax: asm_syntax::InlineAsmX86IntelSyntax = asm_syntax::InlineAsmX86IntelSyntax,
        ModStyle: module_style::ModStyle = module_style::ModStyle::default(),
        DisallowedScriptIdents: disallowed_script_idents::DisallowedScriptIdents = disallowed_script_idents::DisallowedScriptIdents::new(conf),
        OctalEscapes: octal_escapes::OctalEscapes = octal_escapes::OctalEscapes,
        SingleCharLifetimeNames: single_char_lifetime_names::SingleCharLifetimeNames = single_char_lifetime_names::SingleCharLifetimeNames,
        CrateInMacroDef: crate_in_macro_def::CrateInMacroDef = crate_in_macro_def::CrateInMacroDef,
        PubUse: pub_use::PubUse = pub_use::PubUse,
        LargeIncludeFile: large_include_file::LargeIncludeFile = large_include_file::LargeIncludeFile::new(conf),
        DuplicateMod: duplicate_mod::DuplicateMod = duplicate_mod::DuplicateMod::default(),
        UnusedRounding: unused_rounding::UnusedRounding = unused_rounding::UnusedRounding,
        AlmostCompleteRange: almost_complete_range::AlmostCompleteRange = almost_complete_range::AlmostCompleteRange::new(conf),
        MultiAssignments: multi_assignments::MultiAssignments = multi_assignments::MultiAssignments,
        PartialPubFields: partial_pub_fields::PartialPubFields = partial_pub_fields::PartialPubFields,
        UnderscoreTyped: let_with_type_underscore::UnderscoreTyped = let_with_type_underscore::UnderscoreTyped,
        ExcessiveNesting: excessive_nesting::ExcessiveNesting = excessive_nesting::ExcessiveNesting::new(conf),
        NeedlessElse: needless_else::NeedlessElse = needless_else::NeedlessElse,
        RawStrings: raw_strings::RawStrings = raw_strings::RawStrings::new(conf),
        Visibility: visibility::Visibility = visibility::Visibility,
        MultipleBoundLocations: multiple_bound_locations::MultipleBoundLocations = multiple_bound_locations::MultipleBoundLocations,
        FieldScopedVisibilityModifiers: field_scoped_visibility_modifiers::FieldScopedVisibilityModifiers = field_scoped_visibility_modifiers::FieldScopedVisibilityModifiers,
        CfgNotTest: cfg_not_test::CfgNotTest = cfg_not_test::CfgNotTest,
        EmptyLineAfter: empty_line_after::EmptyLineAfter = empty_line_after::EmptyLineAfter::default(),
        InlineTraitBounds: inline_trait_bounds::InlineTraitBounds = inline_trait_bounds::InlineTraitBounds::default(),
        DefinitionInModuleRoot: definition_in_module_root::DefinitionInModuleRoot = definition_in_module_root::DefinitionInModuleRoot::default(),
        // add early passes here, used by `cargo dev new_lint`
    ]]
);

// Fold every late pass into one statically-combined struct (see
// `combined_late_pass`); the method list comes from `late_lint_methods!`.
#[rustfmt::skip]
rustc_lint::late_lint_methods!(
    crate::combined_late_lint_pass,
    [CombinedLateLintPass, (tcx: TyCtxt<'tcx>, conf: &'static Conf, format_args: FormatArgsStorage, attrs: AttrStorage), [
        ArithmeticSideEffects: operators::arithmetic_side_effects::ArithmeticSideEffects = operators::arithmetic_side_effects::ArithmeticSideEffects::new(conf),
        DumpHir: utils::dump_hir::DumpHir = utils::dump_hir::DumpHir,
        Author: utils::author::Author = utils::author::Author,
        AwaitHolding: await_holding_invalid::AwaitHolding = await_holding_invalid::AwaitHolding::new(tcx, conf),
        SerdeApi: serde_api::SerdeApi = serde_api::SerdeApi,
        Types: types::Types = types::Types::new(conf),
        NonminimalBool: booleans::NonminimalBool = booleans::NonminimalBool::new(conf),
        UnportableVariant: enum_clike::UnportableVariant = enum_clike::UnportableVariant,
        FloatLiteral: float_literal::FloatLiteral = float_literal::FloatLiteral::new(conf),
        Ptr: ptr::Ptr = ptr::Ptr,
        NeedlessBool: needless_bool::NeedlessBool = needless_bool::NeedlessBool,
        BoolComparison: bool_comparison::BoolComparison = bool_comparison::BoolComparison,
        NeedlessForEach: needless_for_each::NeedlessForEach = needless_for_each::NeedlessForEach,
        LintPass: misc::LintPass = misc::LintPass,
        EtaReduction: eta_reduction::EtaReduction = eta_reduction::EtaReduction,
        MutMut: mut_mut::MutMut = mut_mut::MutMut::default(),
        UnnecessaryMutPassed: unnecessary_mut_passed::UnnecessaryMutPassed = unnecessary_mut_passed::UnnecessaryMutPassed,
        SignificantDropTightening: significant_drop_tightening::SignificantDropTightening<'tcx> = <significant_drop_tightening::SignificantDropTightening<'_>>::default(),
        LenZero: len_zero::LenZero = len_zero::LenZero::new(conf),
        AssertIsEmpty: assert_is_empty::AssertIsEmpty = assert_is_empty::AssertIsEmpty,
        LenWithoutIsEmpty: len_without_is_empty::LenWithoutIsEmpty = len_without_is_empty::LenWithoutIsEmpty,
        Attributes: attrs::Attributes = attrs::Attributes::new(conf),
        BlocksInConditions: blocks_in_conditions::BlocksInConditions = blocks_in_conditions::BlocksInConditions,
        Unicode: unicode::Unicode = unicode::Unicode,
        UninitVec: uninit_vec::UninitVec = uninit_vec::UninitVec,
        UnitReturnExpectingOrd: unit_return_expecting_ord::UnitReturnExpectingOrd = unit_return_expecting_ord::UnitReturnExpectingOrd,
        StringAdd: strings::StringAdd = strings::StringAdd,
        ImplicitReturn: implicit_return::ImplicitReturn = implicit_return::ImplicitReturn,
        ImplicitSaturatingSub: implicit_saturating_sub::ImplicitSaturatingSub = implicit_saturating_sub::ImplicitSaturatingSub::new(conf),
        DefaultNumericFallback: default_numeric_fallback::DefaultNumericFallback = default_numeric_fallback::DefaultNumericFallback,
        NonOctalUnixPermissions: non_octal_unix_permissions::NonOctalUnixPermissions = non_octal_unix_permissions::NonOctalUnixPermissions,
        ApproxConstant: approx_const::ApproxConstant = approx_const::ApproxConstant::new(conf),
        Matches: matches::Matches = matches::Matches::new(conf),
        ManualNonExhaustive: manual_non_exhaustive::ManualNonExhaustive = manual_non_exhaustive::ManualNonExhaustive::new(conf),
        ManualStrip: manual_strip::ManualStrip = manual_strip::ManualStrip::new(conf),
        CheckedConversions: checked_conversions::CheckedConversions = checked_conversions::CheckedConversions::new(conf),
        MemReplace: mem_replace::MemReplace = mem_replace::MemReplace::new(conf),
        Ranges: ranges::Ranges = ranges::Ranges::new(conf),
        FromOverInto: from_over_into::FromOverInto = from_over_into::FromOverInto::new(conf),
        UseSelf: use_self::UseSelf = use_self::UseSelf::new(conf),
        MissingConstForFn: missing_const_for_fn::MissingConstForFn = missing_const_for_fn::MissingConstForFn::new(conf),
        NeedlessQuestionMark: needless_question_mark::NeedlessQuestionMark = needless_question_mark::NeedlessQuestionMark,
        Casts: casts::Casts = casts::Casts::new(conf),
        SizeOfInElementCount: size_of_in_element_count::SizeOfInElementCount = size_of_in_element_count::SizeOfInElementCount,
        SameNameMethod: same_name_method::SameNameMethod = same_name_method::SameNameMethod,
        IndexRefutableSlice: index_refutable_slice::IndexRefutableSlice = index_refutable_slice::IndexRefutableSlice::new(conf),
        Shadow: shadow::Shadow = <shadow::Shadow>::default(),
        InconsistentStructConstructor: inconsistent_struct_constructor::InconsistentStructConstructor = inconsistent_struct_constructor::InconsistentStructConstructor::new( conf, ),
        Methods: methods::Methods = methods::Methods::new(conf, format_args.clone()),
        UnitTypes: unit_types::UnitTypes = unit_types::UnitTypes::new(format_args.clone()),
        Loops: loops::Loops = loops::Loops::new(conf),
        MainRecursion: main_recursion::MainRecursion = <main_recursion::MainRecursion>::default(),
        Lifetimes: lifetimes::Lifetimes = lifetimes::Lifetimes::new(conf),
        HashMapPass: entry::HashMapPass = entry::HashMapPass,
        MinMaxPass: minmax::MinMaxPass = minmax::MinMaxPass,
        ZeroDiv: zero_div_zero::ZeroDiv = zero_div_zero::ZeroDiv,
        Mutex: mutex_atomic::Mutex = mutex_atomic::Mutex,
        NeedlessUpdate: needless_update::NeedlessUpdate = needless_update::NeedlessUpdate,
        NeedlessBorrowedRef: needless_borrowed_ref::NeedlessBorrowedRef = needless_borrowed_ref::NeedlessBorrowedRef,
        BorrowDerefRef: borrow_deref_ref::BorrowDerefRef = borrow_deref_ref::BorrowDerefRef,
        NoEffect: no_effect::NoEffect = <no_effect::NoEffect>::default(),
        TemporaryAssignment: temporary_assignment::TemporaryAssignment = temporary_assignment::TemporaryAssignment,
        Transmute: transmute::Transmute = transmute::Transmute::new(conf),
        CognitiveComplexity: cognitive_complexity::CognitiveComplexity = cognitive_complexity::CognitiveComplexity::new(conf),
        BoxedLocal: escape::BoxedLocal = escape::BoxedLocal::new(conf),
        UselessVec: useless_vec::UselessVec = useless_vec::UselessVec::new(conf),
        PanicUnimplemented: panic_unimplemented::PanicUnimplemented = panic_unimplemented::PanicUnimplemented::new(conf),
        StringLitAsBytes: strings::StringLitAsBytes = strings::StringLitAsBytes,
        Derive: derive::Derive = derive::Derive,
        DerivableImpls: derivable_impls::DerivableImpls = derivable_impls::DerivableImpls::new(conf),
        DropForgetRef: drop_forget_ref::DropForgetRef = drop_forget_ref::DropForgetRef,
        EmptyEnums: empty_enums::EmptyEnums = empty_enums::EmptyEnums,
        Regex: regex::Regex = <regex::Regex>::default(),
        CopyAndPaste: ifs::CopyAndPaste<'tcx> = ifs::CopyAndPaste::new(tcx, conf),
        CopyIterator: copy_iterator::CopyIterator = copy_iterator::CopyIterator,
        UselessFormat: format::UselessFormat = format::UselessFormat::new(format_args.clone()),
        Swap: swap::Swap = swap::Swap,
        PanickingOverflowChecks: panicking_overflow_checks::PanickingOverflowChecks = panicking_overflow_checks::PanickingOverflowChecks,
        NewWithoutDefault: new_without_default::NewWithoutDefault = <new_without_default::NewWithoutDefault>::default(),
        DisallowedNames: disallowed_names::DisallowedNames = disallowed_names::DisallowedNames::new(conf),
        Functions: functions::Functions = functions::Functions::new(tcx, conf),
        Documentation: doc::Documentation = doc::Documentation::new(conf),
        NegMultiply: neg_multiply::NegMultiply = neg_multiply::NegMultiply,
        LetIfSeq: let_if_seq::LetIfSeq = let_if_seq::LetIfSeq,
        EvalOrderDependence: mixed_read_write_in_expression::EvalOrderDependence = mixed_read_write_in_expression::EvalOrderDependence,
        MissingDoc: missing_doc::MissingDoc = missing_doc::MissingDoc::new(conf),
        MissingInline: missing_inline::MissingInline = missing_inline::MissingInline,
        ExhaustiveItems: exhaustive_items::ExhaustiveItems = exhaustive_items::ExhaustiveItems,
        UnusedResultOk: unused_result_ok::UnusedResultOk = unused_result_ok::UnusedResultOk,
        MatchResultOk: match_result_ok::MatchResultOk = match_result_ok::MatchResultOk,
        PartialEqNeImpl: partialeq_ne_impl::PartialEqNeImpl = partialeq_ne_impl::PartialEqNeImpl,
        UnusedIoAmount: unused_io_amount::UnusedIoAmount = unused_io_amount::UnusedIoAmount,
        LargeEnumVariant: large_enum_variant::LargeEnumVariant = large_enum_variant::LargeEnumVariant::new(conf),
        ExplicitWrite: explicit_write::ExplicitWrite = explicit_write::ExplicitWrite::new(format_args.clone()),
        NeedlessPassByValue: needless_pass_by_value::NeedlessPassByValue = needless_pass_by_value::NeedlessPassByValue,
        PassByRefOrValue: pass_by_ref_or_value::PassByRefOrValue = pass_by_ref_or_value::PassByRefOrValue::new(tcx, conf),
        RefOptionRef: ref_option_ref::RefOptionRef = ref_option_ref::RefOptionRef,
        InfiniteIter: infinite_iter::InfiniteIter = infinite_iter::InfiniteIter,
        InlineFnWithoutBody: inline_fn_without_body::InlineFnWithoutBody = inline_fn_without_body::InlineFnWithoutBody,
        UselessConversion: useless_conversion::UselessConversion = <useless_conversion::UselessConversion>::default(),
        ImplicitHasher: implicit_hasher::ImplicitHasher = implicit_hasher::ImplicitHasher,
        FallibleImplFrom: fallible_impl_from::FallibleImplFrom = fallible_impl_from::FallibleImplFrom,
        QuestionMark: question_mark::QuestionMark = question_mark::QuestionMark::new(conf),
        QuestionMarkUsed: question_mark_used::QuestionMarkUsed = question_mark_used::QuestionMarkUsed,
        SuspiciousImpl: suspicious_trait_impl::SuspiciousImpl = suspicious_trait_impl::SuspiciousImpl,
        MapUnit: map_unit_fn::MapUnit = map_unit_fn::MapUnit,
        MultipleInherentImpl: inherent_impl::MultipleInherentImpl = inherent_impl::MultipleInherentImpl::new(conf),
        NoNegCompOpForPartialOrd: neg_cmp_op_on_partial_ord::NoNegCompOpForPartialOrd = neg_cmp_op_on_partial_ord::NoNegCompOpForPartialOrd,
        Unwrap: unwrap::Unwrap = unwrap::Unwrap::new(conf),
        IndexingSlicing: indexing_slicing::IndexingSlicing = indexing_slicing::IndexingSlicing::new(conf),
        NonCopyConst: non_copy_const::NonCopyConst<'tcx> = non_copy_const::NonCopyConst::new(tcx, conf),
        RedundantClone: redundant_clone::RedundantClone = redundant_clone::RedundantClone,
        SlowVectorInit: slow_vector_initialization::SlowVectorInit = slow_vector_initialization::SlowVectorInit,
        UnnecessaryWraps: unnecessary_wraps::UnnecessaryWraps = unnecessary_wraps::UnnecessaryWraps::new(conf),
        AssertionsOnConstants: assertions_on_constants::AssertionsOnConstants = assertions_on_constants::AssertionsOnConstants::new(conf),
        AssertionsOnResultStates: assertions_on_result_states::AssertionsOnResultStates = assertions_on_result_states::AssertionsOnResultStates,
        InherentToString: inherent_to_string::InherentToString = inherent_to_string::InherentToString,
        TraitBounds: trait_bounds::TraitBounds = trait_bounds::TraitBounds::new(conf),
        ComparisonChain: comparison_chain::ComparisonChain = comparison_chain::ComparisonChain,
        MutableKeyType: mut_key::MutableKeyType<'tcx> = mut_key::MutableKeyType::new(tcx, conf),
        DerefAddrOf: reference::DerefAddrOf = reference::DerefAddrOf,
        FormatImpl: format_impl::FormatImpl = format_impl::FormatImpl::new(format_args.clone()),
        RedundantClosureCall: redundant_closure_call::RedundantClosureCall = redundant_closure_call::RedundantClosureCall,
        UnusedUnit: unused_unit::UnusedUnit = unused_unit::UnusedUnit,
        Return: returns::Return = returns::Return,
        CollapsibleIf: collapsible_if::CollapsibleIf = collapsible_if::CollapsibleIf::new(conf),
        ItemsAfterStatements: items_after_statements::ItemsAfterStatements = items_after_statements::ItemsAfterStatements,
        NeedlessParensOnRangeLiterals: needless_parens_on_range_literals::NeedlessParensOnRangeLiterals = needless_parens_on_range_literals::NeedlessParensOnRangeLiterals,
        NeedlessContinue: needless_continue::NeedlessContinue = needless_continue::NeedlessContinue,
        CreateDir: create_dir::CreateDir = create_dir::CreateDir,
        ItemNameRepetitions: item_name_repetitions::ItemNameRepetitions = item_name_repetitions::ItemNameRepetitions::new(conf),
        UpperCaseAcronyms: upper_case_acronyms::UpperCaseAcronyms = upper_case_acronyms::UpperCaseAcronyms::new(conf),
        Default: default::Default = <default::Default>::default(),
        UnusedSelf: unused_self::UnusedSelf = unused_self::UnusedSelf::new(conf),
        DebugAssertWithMutCall: mutable_debug_assertion::DebugAssertWithMutCall = mutable_debug_assertion::DebugAssertWithMutCall,
        Exit: exit::Exit = exit::Exit,
        ToDigitIsSome: to_digit_is_some::ToDigitIsSome = to_digit_is_some::ToDigitIsSome::new(conf),
        LargeStackArrays: large_stack_arrays::LargeStackArrays = large_stack_arrays::LargeStackArrays::new(conf),
        LargeConstArrays: large_const_arrays::LargeConstArrays = large_const_arrays::LargeConstArrays::new(conf),
        FloatingPointArithmetic: floating_point_arithmetic::FloatingPointArithmetic = floating_point_arithmetic::FloatingPointArithmetic,
        AsConversions: as_conversions::AsConversions = as_conversions::AsConversions,
        LetUnderscore: let_underscore::LetUnderscore = let_underscore::LetUnderscore,
        ExcessiveBools: excessive_bools::ExcessiveBools = excessive_bools::ExcessiveBools::new(conf),
        WildcardImports: wildcard_imports::WildcardImports = wildcard_imports::WildcardImports::new(conf),
        RedundantPubCrate: redundant_pub_crate::RedundantPubCrate = <redundant_pub_crate::RedundantPubCrate>::default(),
        Dereferencing: dereference::Dereferencing<'tcx> = <dereference::Dereferencing<'_>>::default(),
        OptionIfLetElse: option_if_let_else::OptionIfLetElse = option_if_let_else::OptionIfLetElse,
        FutureNotSend: future_not_send::FutureNotSend = future_not_send::FutureNotSend,
        LargeFuture: large_futures::LargeFuture = large_futures::LargeFuture::new(conf),
        IfLetMutex: if_let_mutex::IfLetMutex = if_let_mutex::IfLetMutex,
        IfNotElse: if_not_else::IfNotElse = if_not_else::IfNotElse,
        PatternEquality: equatable_if_let::PatternEquality = equatable_if_let::PatternEquality,
        ManualAsyncFn: manual_async_fn::ManualAsyncFn = manual_async_fn::ManualAsyncFn,
        PanicInResultFn: panic_in_result_fn::PanicInResultFn = panic_in_result_fn::PanicInResultFn,
        MacroUseImports: macro_use::MacroUseImports = <macro_use::MacroUseImports>::default(),
        PatternTypeMismatch: pattern_type_mismatch::PatternTypeMismatch = pattern_type_mismatch::PatternTypeMismatch,
        UnwrapInResult: unwrap_in_result::UnwrapInResult = <unwrap_in_result::UnwrapInResult>::default(),
        SemicolonIfNothingReturned: semicolon_if_nothing_returned::SemicolonIfNothingReturned = semicolon_if_nothing_returned::SemicolonIfNothingReturned,
        AsyncYieldsAsync: async_yields_async::AsyncYieldsAsync = async_yields_async::AsyncYieldsAsync,
        DisallowedMacros: disallowed_macros::DisallowedMacros = disallowed_macros::DisallowedMacros::new(tcx, conf, attrs.clone()),
        DisallowedMethods: disallowed_methods::DisallowedMethods = disallowed_methods::DisallowedMethods::new(tcx, conf),
        EmptyDrop: empty_drop::EmptyDrop = empty_drop::EmptyDrop,
        StrToString: strings::StrToString = strings::StrToString,
        ZeroSizedMapValues: zero_sized_map_values::ZeroSizedMapValues = zero_sized_map_values::ZeroSizedMapValues,
        VecInitThenPush: vec_init_then_push::VecInitThenPush = <vec_init_then_push::VecInitThenPush>::default(),
        RedundantSlicing: redundant_slicing::RedundantSlicing = redundant_slicing::RedundantSlicing,
        FromStrRadix10: from_str_radix_10::FromStrRadix10 = from_str_radix_10::FromStrRadix10,
        IfThenSomeElseNone: if_then_some_else_none::IfThenSomeElseNone = if_then_some_else_none::IfThenSomeElseNone::new(conf),
        BoolAssertComparison: bool_assert_comparison::BoolAssertComparison = bool_assert_comparison::BoolAssertComparison,
        UnusedAsync: unused_async::UnusedAsync = <unused_async::UnusedAsync>::default(),
        DisallowedTypes: disallowed_types::DisallowedTypes = disallowed_types::DisallowedTypes::new(tcx, conf),
        ImportRename: missing_enforced_import_rename::ImportRename = missing_enforced_import_rename::ImportRename::new(tcx, conf),
        StrlenOnCStrings: strlen_on_c_strings::StrlenOnCStrings = strlen_on_c_strings::StrlenOnCStrings::new(conf),
        SelfNamedConstructors: self_named_constructors::SelfNamedConstructors = self_named_constructors::SelfNamedConstructors,
        IterNotReturningIterator: iter_not_returning_iterator::IterNotReturningIterator = iter_not_returning_iterator::IterNotReturningIterator,
        ManualAssert: manual_assert::ManualAssert = manual_assert::ManualAssert,
        NonSendFieldInSendTy: non_send_fields_in_send_ty::NonSendFieldInSendTy = non_send_fields_in_send_ty::NonSendFieldInSendTy::new(conf),
        UndocumentedUnsafeBlocks: undocumented_unsafe_blocks::UndocumentedUnsafeBlocks = undocumented_unsafe_blocks::UndocumentedUnsafeBlocks::new(conf),
        FormatArgs: format_args::FormatArgs<'tcx> = format_args::FormatArgs::new(tcx, conf, format_args.clone()),
        TrailingEmptyArray: trailing_empty_array::TrailingEmptyArray = trailing_empty_array::TrailingEmptyArray,
        NeedlessLateInit: needless_late_init::NeedlessLateInit<'tcx> = needless_late_init::NeedlessLateInit::new(conf),
        ReturnSelfNotMustUse: return_self_not_must_use::ReturnSelfNotMustUse = return_self_not_must_use::ReturnSelfNotMustUse,
        NumberedFields: init_numbered_fields::NumberedFields = init_numbered_fields::NumberedFields,
        ManualBitWidth: bit_width::ManualBitWidth = bit_width::ManualBitWidth::new(conf),
        ManualBits: manual_bits::ManualBits = manual_bits::ManualBits::new(conf),
        DefaultUnionRepresentation: default_union_representation::DefaultUnionRepresentation = default_union_representation::DefaultUnionRepresentation,
        OnlyUsedInRecursion: only_used_in_recursion::OnlyUsedInRecursion = <only_used_in_recursion::OnlyUsedInRecursion>::default(),
        DbgMacro: dbg_macro::DbgMacro = dbg_macro::DbgMacro::new(conf),
        Write: write::Write = write::Write::new(conf, format_args.clone()),
        Cargo: cargo::Cargo = cargo::Cargo::new(conf),
        EmptyWithBrackets: empty_with_brackets::EmptyWithBrackets = empty_with_brackets::EmptyWithBrackets::default(),
        UnnecessaryOwnedEmptyStrings: unnecessary_owned_empty_strings::UnnecessaryOwnedEmptyStrings = unnecessary_owned_empty_strings::UnnecessaryOwnedEmptyStrings,
        FormatPushString: format_push_string::FormatPushString = format_push_string::FormatPushString::new(format_args.clone()),
        LargeIncludeFile: large_include_file::LargeIncludeFile = large_include_file::LargeIncludeFile::new(conf),
        TrimSplitWhitespace: strings::TrimSplitWhitespace = strings::TrimSplitWhitespace,
        RcCloneInVecInit: rc_clone_in_vec_init::RcCloneInVecInit = rc_clone_in_vec_init::RcCloneInVecInit,
        SwapPtrToRef: swap_ptr_to_ref::SwapPtrToRef = swap_ptr_to_ref::SwapPtrToRef,
        TypeParamMismatch: mismatching_type_param_order::TypeParamMismatch = mismatching_type_param_order::TypeParamMismatch,
        ReadZeroByteVec: read_zero_byte_vec::ReadZeroByteVec = read_zero_byte_vec::ReadZeroByteVec,
        DefaultIterEmpty: default_instead_of_iter_empty::DefaultIterEmpty = default_instead_of_iter_empty::DefaultIterEmpty,
        ManualRemEuclid: manual_rem_euclid::ManualRemEuclid = manual_rem_euclid::ManualRemEuclid::new(conf),
        ManualRetain: manual_retain::ManualRetain = manual_retain::ManualRetain::new(conf),
        ManualRotate: manual_rotate::ManualRotate = manual_rotate::ManualRotate,
        Operators: operators::Operators = operators::Operators::new(conf),
        StdReexports: std_instead_of_core::StdReexports = std_instead_of_core::StdReexports::new(conf),
        UncheckedTimeSubtraction: time_subtraction::UncheckedTimeSubtraction = time_subtraction::UncheckedTimeSubtraction::new(conf),
        PartialeqToNone: partialeq_to_none::PartialeqToNone = partialeq_to_none::PartialeqToNone,
        ManualAbsDiff: manual_abs_diff::ManualAbsDiff = manual_abs_diff::ManualAbsDiff::new(conf),
        ManualClamp: manual_clamp::ManualClamp = manual_clamp::ManualClamp::new(conf),
        ManualStringNew: manual_string_new::ManualStringNew = manual_string_new::ManualStringNew,
        UnusedPeekable: unused_peekable::UnusedPeekable = unused_peekable::UnusedPeekable,
        BoolToIntWithIf: bool_to_int_with_if::BoolToIntWithIf = bool_to_int_with_if::BoolToIntWithIf,
        BoxDefault: box_default::BoxDefault = box_default::BoxDefault,
        ImplicitSaturatingAdd: implicit_saturating_add::ImplicitSaturatingAdd = implicit_saturating_add::ImplicitSaturatingAdd,
        MissingTraitMethods: missing_trait_methods::MissingTraitMethods = missing_trait_methods::MissingTraitMethods::new(conf),
        FromRawWithVoidPtr: from_raw_with_void_ptr::FromRawWithVoidPtr = from_raw_with_void_ptr::FromRawWithVoidPtr,
        ConfusingXorAndPow: suspicious_xor_used_as_pow::ConfusingXorAndPow = suspicious_xor_used_as_pow::ConfusingXorAndPow,
        ManualIsAsciiCheck: manual_is_ascii_check::ManualIsAsciiCheck = manual_is_ascii_check::ManualIsAsciiCheck::new(conf),
        SemicolonBlock: semicolon_block::SemicolonBlock = semicolon_block::SemicolonBlock::new(conf),
        PermissionsSetReadonlyFalse: permissions_set_readonly_false::PermissionsSetReadonlyFalse = permissions_set_readonly_false::PermissionsSetReadonlyFalse,
        SizeOfRef: size_of_ref::SizeOfRef = size_of_ref::SizeOfRef,
        MultipleUnsafeOpsPerBlock: multiple_unsafe_ops_per_block::MultipleUnsafeOpsPerBlock = multiple_unsafe_ops_per_block::MultipleUnsafeOpsPerBlock,
        ExtraUnusedTypeParameters: extra_unused_type_parameters::ExtraUnusedTypeParameters = extra_unused_type_parameters::ExtraUnusedTypeParameters::new(conf),
        NoMangleWithRustAbi: no_mangle_with_rust_abi::NoMangleWithRustAbi = no_mangle_with_rust_abi::NoMangleWithRustAbi,
        CollectionIsNeverRead: collection_is_never_read::CollectionIsNeverRead = collection_is_never_read::CollectionIsNeverRead,
        MissingAssertMessage: missing_assert_message::MissingAssertMessage = missing_assert_message::MissingAssertMessage,
        NeedlessMaybeSized: needless_maybe_sized::NeedlessMaybeSized = needless_maybe_sized::NeedlessMaybeSized,
        RedundantAsyncBlock: redundant_async_block::RedundantAsyncBlock = redundant_async_block::RedundantAsyncBlock,
        ManualMainSeparatorStr: manual_main_separator_str::ManualMainSeparatorStr = manual_main_separator_str::ManualMainSeparatorStr::new(conf),
        UnnecessaryStruct: unnecessary_struct_initialization::UnnecessaryStruct = unnecessary_struct_initialization::UnnecessaryStruct,
        UnnecessaryBoxReturns: unnecessary_box_returns::UnnecessaryBoxReturns = unnecessary_box_returns::UnnecessaryBoxReturns::new(conf),
        TestsOutsideTestModule: tests_outside_test_module::TestsOutsideTestModule = tests_outside_test_module::TestsOutsideTestModule,
        ManualSliceSizeCalculation: manual_slice_size_calculation::ManualSliceSizeCalculation = manual_slice_size_calculation::ManualSliceSizeCalculation::new(conf),
        ItemsAfterTestModule: items_after_test_module::ItemsAfterTestModule = items_after_test_module::ItemsAfterTestModule,
        DefaultConstructedUnitStructs: default_constructed_unit_structs::DefaultConstructedUnitStructs = default_constructed_unit_structs::DefaultConstructedUnitStructs,
        MissingFieldsInDebug: missing_fields_in_debug::MissingFieldsInDebug = missing_fields_in_debug::MissingFieldsInDebug,
        EndianBytes: endian_bytes::EndianBytes = endian_bytes::EndianBytes,
        RedundantTypeAnnotations: redundant_type_annotations::RedundantTypeAnnotations = redundant_type_annotations::RedundantTypeAnnotations,
        ArcWithNonSendSync: arc_with_non_send_sync::ArcWithNonSendSync = arc_with_non_send_sync::ArcWithNonSendSync,
        NeedlessIfs: needless_ifs::NeedlessIfs = needless_ifs::NeedlessIfs,
        MinIdentChars: min_ident_chars::MinIdentChars = min_ident_chars::MinIdentChars::new(conf),
        LargeStackFrames: large_stack_frames::LargeStackFrames = large_stack_frames::LargeStackFrames::new(conf),
        SingleRangeInVecInit: single_range_in_vec_init::SingleRangeInVecInit = single_range_in_vec_init::SingleRangeInVecInit,
        NeedlessPassByRefMut: needless_pass_by_ref_mut::NeedlessPassByRefMut<'tcx> = needless_pass_by_ref_mut::NeedlessPassByRefMut::new(conf),
        NonCanonicalImpls: non_canonical_impls::NonCanonicalImpls = non_canonical_impls::NonCanonicalImpls::new(tcx),
        SingleCallFn: single_call_fn::SingleCallFn = single_call_fn::SingleCallFn::new(conf),
        LegacyNumericConstants: legacy_numeric_constants::LegacyNumericConstants = legacy_numeric_constants::LegacyNumericConstants::new(conf),
        ManualRangePatterns: manual_range_patterns::ManualRangePatterns = manual_range_patterns::ManualRangePatterns,
        TupleArrayConversions: tuple_array_conversions::TupleArrayConversions = tuple_array_conversions::TupleArrayConversions::new(conf),
        ManualFloatMethods: manual_float_methods::ManualFloatMethods = manual_float_methods::ManualFloatMethods::new(conf),
        FourForwardSlashes: four_forward_slashes::FourForwardSlashes = four_forward_slashes::FourForwardSlashes,
        ErrorImplError: error_impl_error::ErrorImplError = error_impl_error::ErrorImplError,
        AbsolutePaths: absolute_paths::AbsolutePaths = absolute_paths::AbsolutePaths::new(conf),
        RedundantLocals: redundant_locals::RedundantLocals = redundant_locals::RedundantLocals,
        IgnoredUnitPatterns: ignored_unit_patterns::IgnoredUnitPatterns = ignored_unit_patterns::IgnoredUnitPatterns,
        ReserveAfterInitialization: reserve_after_initialization::ReserveAfterInitialization = <reserve_after_initialization::ReserveAfterInitialization>::default(),
        ImpliedBoundsInImpls: implied_bounds_in_impls::ImpliedBoundsInImpls = implied_bounds_in_impls::ImpliedBoundsInImpls,
        MissingAssertsForIndexing: missing_asserts_for_indexing::MissingAssertsForIndexing = missing_asserts_for_indexing::MissingAssertsForIndexing,
        UnnecessaryMapOnConstructor: unnecessary_map_on_constructor::UnnecessaryMapOnConstructor = unnecessary_map_on_constructor::UnnecessaryMapOnConstructor,
        NeedlessBorrowsForGenericArgs: needless_borrows_for_generic_args::NeedlessBorrowsForGenericArgs<'tcx> = needless_borrows_for_generic_args::NeedlessBorrowsForGenericArgs::new( conf, ),
        ManualHashOne: manual_hash_one::ManualHashOne = manual_hash_one::ManualHashOne::new(conf),
        IterWithoutIntoIter: iter_without_into_iter::IterWithoutIntoIter = iter_without_into_iter::IterWithoutIntoIter,
        PathbufThenPush: pathbuf_init_then_push::PathbufThenPush<'tcx> = <pathbuf_init_then_push::PathbufThenPush<'_>>::default(),
        IterOverHashType: iter_over_hash_type::IterOverHashType = iter_over_hash_type::IterOverHashType,
        ImplHashWithBorrowStrBytes: impl_hash_with_borrow_str_and_bytes::ImplHashWithBorrowStrBytes = impl_hash_with_borrow_str_and_bytes::ImplHashWithBorrowStrBytes,
        RepeatVecWithCapacity: repeat_vec_with_capacity::RepeatVecWithCapacity = repeat_vec_with_capacity::RepeatVecWithCapacity::new(conf),
        UninhabitedReferences: uninhabited_references::UninhabitedReferences = uninhabited_references::UninhabitedReferences,
        IneffectiveOpenOptions: ineffective_open_options::IneffectiveOpenOptions = ineffective_open_options::IneffectiveOpenOptions,
        UnconditionalRecursion: unconditional_recursion::UnconditionalRecursion = <unconditional_recursion::UnconditionalRecursion>::default(),
        PubUnderscoreFields: pub_underscore_fields::PubUnderscoreFields = pub_underscore_fields::PubUnderscoreFields::new(conf),
        MissingConstForThreadLocal: missing_const_for_thread_local::MissingConstForThreadLocal = missing_const_for_thread_local::MissingConstForThreadLocal::new(conf),
        IncompatibleMsrv: incompatible_msrv::IncompatibleMsrv = incompatible_msrv::IncompatibleMsrv::new(tcx, conf),
        ToStringTraitImpl: to_string_trait_impl::ToStringTraitImpl = to_string_trait_impl::ToStringTraitImpl,
        AssigningClones: assigning_clones::AssigningClones = assigning_clones::AssigningClones::new(conf),
        ZeroRepeatSideEffects: zero_repeat_side_effects::ZeroRepeatSideEffects = zero_repeat_side_effects::ZeroRepeatSideEffects,
        ExprMetavarsInUnsafe: macro_metavars_in_unsafe::ExprMetavarsInUnsafe = macro_metavars_in_unsafe::ExprMetavarsInUnsafe::new(conf),
        StringPatterns: string_patterns::StringPatterns = string_patterns::StringPatterns::new(conf),
        SetContainsOrInsert: set_contains_or_insert::SetContainsOrInsert = set_contains_or_insert::SetContainsOrInsert,
        ZombieProcesses: zombie_processes::ZombieProcesses = zombie_processes::ZombieProcesses,
        PointersInNomemAsmBlock: pointers_in_nomem_asm_block::PointersInNomemAsmBlock = pointers_in_nomem_asm_block::PointersInNomemAsmBlock,
        ManualIsPowerOfTwo: manual_is_power_of_two::ManualIsPowerOfTwo = manual_is_power_of_two::ManualIsPowerOfTwo::new(conf),
        NonZeroSuggestions: non_zero_suggestions::NonZeroSuggestions = non_zero_suggestions::NonZeroSuggestions,
        LiteralStringWithFormattingArg: literal_string_with_formatting_args::LiteralStringWithFormattingArg = literal_string_with_formatting_args::LiteralStringWithFormattingArg,
        UnusedTraitNames: unused_trait_names::UnusedTraitNames = unused_trait_names::UnusedTraitNames::new(conf),
        ManualIgnoreCaseCmp: manual_ignore_case_cmp::ManualIgnoreCaseCmp = manual_ignore_case_cmp::ManualIgnoreCaseCmp,
        UnnecessaryLiteralBound: unnecessary_literal_bound::UnnecessaryLiteralBound = unnecessary_literal_bound::UnnecessaryLiteralBound,
        ArbitrarySourceItemOrdering: arbitrary_source_item_ordering::ArbitrarySourceItemOrdering = arbitrary_source_item_ordering::ArbitrarySourceItemOrdering::new(tcx, conf),
        UselessConcat: useless_concat::UselessConcat = useless_concat::UselessConcat,
        UnneededStructPattern: unneeded_struct_pattern::UnneededStructPattern = unneeded_struct_pattern::UnneededStructPattern,
        UnnecessarySemicolon: unnecessary_semicolon::UnnecessarySemicolon = <unnecessary_semicolon::UnnecessarySemicolon>::default(),
        NonStdLazyStatic: non_std_lazy_statics::NonStdLazyStatic = non_std_lazy_statics::NonStdLazyStatic::new(conf),
        ManualOptionAsSlice: manual_option_as_slice::ManualOptionAsSlice = manual_option_as_slice::ManualOptionAsSlice::new(conf),
        SingleOptionMap: single_option_map::SingleOptionMap = single_option_map::SingleOptionMap,
        RedundantTestPrefix: redundant_test_prefix::RedundantTestPrefix = redundant_test_prefix::RedundantTestPrefix,
        ClonedRefToSliceRefs: cloned_ref_to_slice_refs::ClonedRefToSliceRefs = cloned_ref_to_slice_refs::ClonedRefToSliceRefs::new(conf),
        InfallibleTryFrom: infallible_try_from::InfallibleTryFrom = infallible_try_from::InfallibleTryFrom,
        CoerceContainerToAny: coerce_container_to_any::CoerceContainerToAny = coerce_container_to_any::CoerceContainerToAny,
        ToplevelRefArg: toplevel_ref_arg::ToplevelRefArg = toplevel_ref_arg::ToplevelRefArg,
        VolatileComposites: volatile_composites::VolatileComposites = volatile_composites::VolatileComposites,
        ReplaceBox: replace_box::ReplaceBox = <replace_box::ReplaceBox>::default(),
        DisallowedFields: disallowed_fields::DisallowedFields = disallowed_fields::DisallowedFields::new(tcx, conf),
        ManualIlog2: manual_ilog2::ManualIlog2 = manual_ilog2::ManualIlog2::new(conf),
        SameLengthAndCapacity: same_length_and_capacity::SameLengthAndCapacity = same_length_and_capacity::SameLengthAndCapacity,
        DurationSuboptimalUnits: duration_suboptimal_units::DurationSuboptimalUnits = duration_suboptimal_units::DurationSuboptimalUnits::new(tcx, conf),
        ManualTake: manual_take::ManualTake = manual_take::ManualTake::new(conf),
        ManualCheckedOps: manual_checked_ops::ManualCheckedOps = manual_checked_ops::ManualCheckedOps,
        ManualPopIf: manual_pop_if::ManualPopIf = manual_pop_if::ManualPopIf::new(tcx, conf),
        ManualNoopWaker: manual_noop_waker::ManualNoopWaker = manual_noop_waker::ManualNoopWaker::new(conf),
        ByteCharSlice: byte_char_slices::ByteCharSlice = byte_char_slices::ByteCharSlice,
        ManualAssertEq: manual_assert_eq::ManualAssertEq = manual_assert_eq::ManualAssertEq,
        WithCapacityZero: with_capacity_zero::WithCapacityZero = with_capacity_zero::WithCapacityZero,
        RefPatterns: ref_patterns::RefPatterns = ref_patterns::RefPatterns,
        RedundantElse: redundant_else::RedundantElse = redundant_else::RedundantElse,
        RestWhenDestructuringStruct: rest_when_destructuring_struct::RestWhenDestructuringStruct = rest_when_destructuring_struct::RestWhenDestructuringStruct,
        BlockScrutinee: block_scrutinee::BlockScrutinee = block_scrutinee::BlockScrutinee,
        NonnullUncheckedOnBoxPtr: nonnull_unchecked_on_box_ptr::NonnullUncheckedOnBoxPtr = nonnull_unchecked_on_box_ptr::NonnullUncheckedOnBoxPtr::new(conf),
        NeedlessNonzeroGet: needless_nonzero_get::NeedlessNonzeroGet = needless_nonzero_get::NeedlessNonzeroGet::new(conf),
        // add late passes here, used by `cargo dev new_lint`
    ]]
);
