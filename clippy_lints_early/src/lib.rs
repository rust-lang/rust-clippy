#![feature(array_windows, box_patterns, if_let_guard, macro_metavar_expr_concat, rustc_private)]
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

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
// extern crate rustc_hir_typeck;
extern crate rustc_lexer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate thin_vec;

#[macro_use]
extern crate declare_clippy_lint;

#[macro_use]
extern crate clippy_utils;

pub mod declared_lints;

// begin lints modules, do not remove this comment, it's used in `update_lints`
mod almost_complete_range;
mod asm_syntax;
mod byte_char_slices;
mod cfg_not_test;
mod crate_in_macro_def;
mod disallowed_script_idents;
mod double_parens;
mod duplicate_mod;
mod else_if_without_else;
mod empty_line_after;
mod excessive_nesting;
mod field_scoped_visibility_modifiers;
mod formatting;
mod int_plus_one;
mod literal_representation;
mod module_style;
mod multi_assignments;
mod multiple_bound_locations;
mod needless_arbitrary_self_type;
mod needless_continue;
mod needless_else;
mod non_expressive_names;
mod nonstandard_macro_braces;
mod octal_escapes;
mod option_env_unwrap;
mod partial_pub_fields;
mod precedence;
mod pub_use;
mod raw_strings;
mod redundant_else;
mod redundant_field_names;
mod redundant_static_lifetimes;
mod single_char_lifetime_names;
mod single_component_path_imports;
mod suspicious_operation_groupings;
mod tabs_in_doc_comments;
mod unnecessary_self_imports;
mod unnested_or_patterns;
mod unsafe_removed_from_name;
mod unused_rounding;
mod visibility;
// end lints modules, do not remove this comment, it's used in `update_lints`

use clippy_config::Conf;
use rustc_lint::LintStore;

pub fn register_lint_passes(store: &mut LintStore, conf: &'static Conf) {
    store.register_early_pass(|| Box::new(unnecessary_self_imports::UnnecessarySelfImports));
    store.register_early_pass(move || Box::new(redundant_static_lifetimes::RedundantStaticLifetimes::new(conf)));
    store.register_early_pass(move || Box::new(redundant_field_names::RedundantFieldNames::new(conf)));
    store.register_early_pass(move || Box::new(unnested_or_patterns::UnnestedOrPatterns::new(conf)));
    store.register_early_pass(|| Box::new(empty_line_after::EmptyLineAfter::new()));
    store.register_early_pass(|| Box::new(byte_char_slices::ByteCharSlice));
    store.register_early_pass(|| Box::new(cfg_not_test::CfgNotTest));
    store.register_early_pass(|| Box::new(field_scoped_visibility_modifiers::FieldScopedVisibilityModifiers));
    store.register_early_pass(|| Box::new(multiple_bound_locations::MultipleBoundLocations));
    store.register_early_pass(|| Box::new(visibility::Visibility));
    store.register_early_pass(move || Box::new(raw_strings::RawStrings::new(conf)));
    store.register_early_pass(|| Box::new(needless_else::NeedlessElse));
    store.register_early_pass(move || Box::new(excessive_nesting::ExcessiveNesting::new(conf)));
    store.register_early_pass(|| Box::new(partial_pub_fields::PartialPubFields));
    store.register_early_pass(|| Box::new(multi_assignments::MultiAssignments));
    store.register_early_pass(|| Box::<duplicate_mod::DuplicateMod>::default());
    store.register_early_pass(|| Box::new(unused_rounding::UnusedRounding));
    store.register_early_pass(move || Box::new(almost_complete_range::AlmostCompleteRange::new(conf)));
    store.register_early_pass(|| Box::new(pub_use::PubUse));
    store.register_early_pass(|| Box::new(crate_in_macro_def::CrateInMacroDef));
    store.register_early_pass(|| Box::new(single_char_lifetime_names::SingleCharLifetimeNames));
    store.register_early_pass(|| Box::new(octal_escapes::OctalEscapes));
    store.register_early_pass(move || Box::new(disallowed_script_idents::DisallowedScriptIdents::new(conf)));
    store.register_early_pass(move || Box::new(module_style::ModStyle));
    store.register_early_pass(|| Box::new(asm_syntax::InlineAsmX86AttSyntax));
    store.register_early_pass(|| Box::new(asm_syntax::InlineAsmX86IntelSyntax));
    store.register_early_pass(move || Box::new(non_expressive_names::NonExpressiveNames::new(conf)));
    store.register_early_pass(move || Box::new(nonstandard_macro_braces::MacroBraces::new(conf)));
    store.register_early_pass(|| Box::new(option_env_unwrap::OptionEnvUnwrap));
    store.register_early_pass(|| Box::<single_component_path_imports::SingleComponentPathImports>::default());
    store.register_early_pass(|| Box::new(tabs_in_doc_comments::TabsInDocComments));
    store.register_early_pass(|| Box::new(needless_arbitrary_self_type::NeedlessArbitrarySelfType));
    store.register_early_pass(move || Box::new(literal_representation::LiteralDigitGrouping::new(conf)));
    store.register_early_pass(move || Box::new(literal_representation::DecimalLiteralRepresentation::new(conf)));
    store.register_early_pass(|| Box::new(needless_continue::NeedlessContinue));
    store.register_early_pass(|| Box::new(redundant_else::RedundantElse));
    store.register_early_pass(|| Box::new(precedence::Precedence));
    store.register_early_pass(|| Box::new(unsafe_removed_from_name::UnsafeNameRemoval));
    store.register_early_pass(|| Box::new(else_if_without_else::ElseIfWithoutElse));
    store.register_early_pass(|| Box::new(int_plus_one::IntPlusOne));
    store.register_early_pass(|| Box::new(formatting::Formatting));
    store.register_early_pass(|| Box::new(double_parens::DoubleParens));
    store.register_early_pass(|| Box::new(suspicious_operation_groupings::SuspiciousOperationGroupings));
    // add lints here, do not remove this comment, it's used in `new_lint`
}
