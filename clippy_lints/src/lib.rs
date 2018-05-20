// error-pattern:cargo-clippy

#![feature(box_syntax)]
#![feature(rustc_private)]
#![feature(slice_patterns)]
#![feature(stmt_expr_attributes)]
#![feature(conservative_impl_trait)]
#![feature(range_contains)]
#![feature(macro_vis_matcher)]
#![allow(unknown_lints, indexing_slicing, shadow_reuse, missing_docs_in_private_items)]
#![recursion_limit = "256"]
// FIXME(mark-i-m) remove after i128 stablization merges
#![allow(stable_features)]
#![feature(i128, i128_type)]
#![feature(iterator_find_map)]


extern crate cargo_metadata;
#[macro_use]
extern crate rustc;
extern crate rustc_typeck;
extern crate rustc_target;
extern crate syntax;
extern crate syntax_pos;

extern crate toml;

// for unicode nfc normalization

extern crate unicode_normalization;

// for semver check in attrs.rs

extern crate semver;

// for regex checking

extern crate regex_syntax;

// for finding minimal boolean expressions

extern crate quine_mc_cluskey;

extern crate rustc_errors;

#[macro_use]
extern crate matches as matches_macro;

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

extern crate itertools;
extern crate pulldown_cmark;
extern crate url;

#[macro_use]
extern crate if_chain;

macro_rules! declare_clippy_lint {
    { pub $name:tt, style, $description:tt } => {
        declare_lint! { pub $name, Warn, $description }
    };
    { pub $name:tt, correctness, $description:tt } => {
        declare_lint! { pub $name, Deny, $description }
    };
    { pub $name:tt, complexity, $description:tt } => {
        declare_lint! { pub $name, Warn, $description }
    };
    { pub $name:tt, perf, $description:tt } => {
        declare_lint! { pub $name, Warn, $description }
    };
    { pub $name:tt, pedantic, $description:tt } => {
        declare_lint! { pub $name, Allow, $description }
    };
    { pub $name:tt, restriction, $description:tt } => {
        declare_lint! { pub $name, Allow, $description }
    };
    { pub $name:tt, cargo, $description:tt } => {
        declare_lint! { pub $name, Allow, $description }
    };
    { pub $name:tt, nursery, $description:tt } => {
        declare_lint! { pub $name, Allow, $description }
    };
    { pub $name:tt, internal, $description:tt } => {
        declare_lint! { pub $name, Allow, $description }
    };
    { pub $name:tt, internal_warn, $description:tt } => {
        declare_lint! { pub $name, Warn, $description }
    };
}

pub mod consts;
#[macro_use]
pub mod utils;

// begin lints modules, do not remove this comment, it’s used in `update_lints`
pub mod approx_const;
pub mod arithmetic;
pub mod array_indexing;
pub mod assign_ops;
pub mod attrs;
pub mod bit_mask;
pub mod blacklisted_name;
pub mod block_in_if_condition;
pub mod booleans;
pub mod bytecount;
pub mod collapsible_if;
pub mod const_static_lifetime;
pub mod copies;
pub mod cyclomatic_complexity;
pub mod derive;
pub mod doc;
pub mod double_comparison;
pub mod double_parens;
pub mod drop_forget_ref;
pub mod else_if_without_else;
pub mod empty_enum;
pub mod entry;
pub mod enum_clike;
pub mod enum_glob_use;
pub mod enum_variants;
pub mod eq_op;
pub mod erasing_op;
pub mod escape;
pub mod eta_reduction;
pub mod eval_order_dependence;
pub mod excessive_precision;
pub mod explicit_write;
pub mod fallible_impl_from;
pub mod format;
pub mod formatting;
pub mod functions;
pub mod identity_conversion;
pub mod identity_op;
pub mod if_let_redundant_pattern_matching;
pub mod if_not_else;
pub mod infallible_destructuring_match;
pub mod infinite_iter;
pub mod inline_fn_without_body;
pub mod int_plus_one;
pub mod invalid_ref;
pub mod items_after_statements;
pub mod large_enum_variant;
pub mod len_zero;
pub mod let_if_seq;
pub mod lifetimes;
pub mod literal_representation;
pub mod loops;
pub mod map_clone;
pub mod map_unit_fn;
pub mod matches;
pub mod mem_forget;
pub mod methods;
pub mod minmax;
pub mod misc;
pub mod misc_early;
pub mod missing_doc;
pub mod multiple_crate_versions;
pub mod mut_mut;
pub mod mut_reference;
pub mod mutex_atomic;
pub mod needless_bool;
pub mod needless_borrow;
pub mod needless_borrowed_ref;
pub mod needless_continue;
pub mod needless_pass_by_value;
pub mod needless_update;
pub mod neg_multiply;
pub mod new_without_default;
pub mod no_effect;
pub mod non_expressive_names;
pub mod ok_if_let;
pub mod open_options;
pub mod overflow_check_conditional;
pub mod panic;
pub mod partialeq_ne_impl;
pub mod precedence;
pub mod ptr;
pub mod question_mark;
pub mod ranges;
pub mod redundant_field_names;
pub mod reference;
pub mod regex;
pub mod replace_consts;
pub mod returns;
pub mod serde_api;
pub mod shadow;
pub mod strings;
pub mod suspicious_trait_impl;
pub mod swap;
pub mod temporary_assignment;
pub mod transmute;
pub mod types;
pub mod unicode;
pub mod unsafe_removed_from_name;
pub mod unused_io_amount;
pub mod unused_label;
pub mod use_self;
pub mod vec;
pub mod write;
pub mod zero_div_zero;
// end lints modules, do not remove this comment, it’s used in `update_lints`

mod reexport {
    pub use syntax::ast::{Name, NodeId};
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn register_lints(sess: &rustc::session::Session, store: &mut rustc::lint::LintStore) {
    // look for `clippy.toml` but
    // do not require the file to exist
    let file_name = match utils::conf::lookup_conf_file() {
        Ok(path) => path,
        Err(error) => {
            sess.struct_err(&format!("error finding Clippy's configuration file: {}", error)).emit();
            None
        }
    };

    let file_name = file_name.map(|file_name| if file_name.is_relative() {
        sess
            .local_crate_source_file
            .as_ref()
            .and_then(|file| std::path::Path::new(&file).parent().map(std::path::Path::to_path_buf))
            .unwrap_or_default()
            .join(file_name)
    } else {
        file_name
    });

    let (conf, errors) = utils::conf::read(file_name.as_ref().map(|p| p.as_ref()));

    // all conf errors are non-fatal, we just use the default conf in case of error
    for error in errors {
        sess.struct_err(&format!("error reading Clippy's configuration file `{}`: {}", file_name.as_ref().and_then(|p| p.to_str()).unwrap_or(""), error)).emit();
    }

    // begin deprecated lints, do not remove this comment, it’s used in `update_lints`
    store.register_removed(
        "should_assert_eq",
        "`assert!()` will be more flexible with RFC 2011",
    );
    store.register_removed(
        "extend_from_slice",
        "`.extend_from_slice(_)` is a faster way to extend a Vec by a slice",
    );
    store.register_removed(
        "range_step_by_zero",
        "`iterator.step_by(0)` panics nowadays",
    );
    store.register_removed(
        "unstable_as_slice",
        "`Vec::as_slice` has been stabilized in 1.7",
    );
    store.register_removed(
        "unstable_as_mut_slice",
        "`Vec::as_mut_slice` has been stabilized in 1.7",
    );
    store.register_removed(
        "str_to_string",
        "using `str::to_string` is common even today and specialization will likely happen soon",
    );
    store.register_removed(
        "string_to_string",
        "using `string::to_string` is common even today and specialization will likely happen soon",
    );
    store.register_removed(
        "misaligned_transmute",
        "this lint has been split into cast_ptr_alignment and transmute_ptr_to_ptr",
    );
    // end deprecated lints, do not remove this comment, it’s used in `update_lints`

    store.register_late_pass(None, false, box serde_api::Serde);
    store.register_early_pass(None, false, box utils::internal_lints::Clippy);
    store.register_late_pass(None, false, box utils::internal_lints::LintWithoutLintPass::default());
    store.register_late_pass(None, false, box utils::inspector::Pass);
    store.register_late_pass(None, false, box utils::author::Pass);
    store.register_late_pass(None, false, box types::TypePass);
    store.register_late_pass(None, false, box booleans::NonminimalBool);
    store.register_late_pass(None, false, box eq_op::EqOp);
    store.register_early_pass(None, false, box enum_variants::EnumVariantNames::new(conf.enum_variant_name_threshold));
    store.register_late_pass(None, false, box enum_glob_use::EnumGlobUse);
    store.register_late_pass(None, false, box enum_clike::UnportableVariant);
    store.register_late_pass(None, false, box excessive_precision::ExcessivePrecision);
    store.register_late_pass(None, false, box bit_mask::BitMask::new(conf.verbose_bit_mask_threshold));
    store.register_late_pass(None, false, box ptr::PointerPass);
    store.register_late_pass(None, false, box needless_bool::NeedlessBool);
    store.register_late_pass(None, false, box needless_bool::BoolComparison);
    store.register_late_pass(None, false, box approx_const::Pass);
    store.register_late_pass(None, false, box misc::Pass);
    store.register_early_pass(None, false, box precedence::Precedence);
    store.register_early_pass(None, false, box needless_continue::NeedlessContinue);
    store.register_late_pass(None, false, box eta_reduction::EtaPass);
    store.register_late_pass(None, false, box identity_op::IdentityOp);
    store.register_late_pass(None, false, box erasing_op::ErasingOp);
    store.register_early_pass(None, false, box items_after_statements::ItemsAfterStatements);
    store.register_late_pass(None, false, box mut_mut::MutMut);
    store.register_late_pass(None, false, box mut_reference::UnnecessaryMutPassed);
    store.register_late_pass(None, false, box len_zero::LenZero);
    store.register_late_pass(None, false, box attrs::AttrPass);
    store.register_early_pass(None, false, box collapsible_if::CollapsibleIf);
    store.register_late_pass(None, false, box block_in_if_condition::BlockInIfCondition);
    store.register_late_pass(None, false, box unicode::Unicode);
    store.register_late_pass(None, false, box strings::StringAdd);
    store.register_early_pass(None, false, box returns::ReturnPass);
    store.register_late_pass(None, false, box methods::Pass);
    store.register_late_pass(None, false, box shadow::Pass);
    store.register_late_pass(None, false, box types::LetPass);
    store.register_late_pass(None, false, box types::UnitCmp);
    store.register_late_pass(None, false, box loops::Pass);
    store.register_late_pass(None, false, box lifetimes::LifetimePass);
    store.register_late_pass(None, false, box entry::HashMapLint);
    store.register_late_pass(None, false, box ranges::Pass);
    store.register_late_pass(None, false, box types::CastPass);
    store.register_late_pass(None, false, box types::TypeComplexityPass::new(conf.type_complexity_threshold));
    store.register_late_pass(None, false, box matches::MatchPass);
    store.register_late_pass(None, false, box minmax::MinMaxPass);
    store.register_late_pass(None, false, box open_options::NonSensical);
    store.register_late_pass(None, false, box zero_div_zero::Pass);
    store.register_late_pass(None, false, box mutex_atomic::MutexAtomic);
    store.register_late_pass(None, false, box needless_update::Pass);
    store.register_late_pass(None, false, box needless_borrow::NeedlessBorrow);
    store.register_late_pass(None, false, box needless_borrowed_ref::NeedlessBorrowedRef);
    store.register_late_pass(None, false, box no_effect::Pass);
    store.register_late_pass(None, false, box map_clone::Pass);
    store.register_late_pass(None, false, box temporary_assignment::Pass);
    store.register_late_pass(None, false, box transmute::Transmute);
    store.register_late_pass(
        None,
        false,
        box cyclomatic_complexity::CyclomaticComplexity::new(conf.cyclomatic_complexity_threshold)
    );
    store.register_late_pass(None, false, box escape::Pass{too_large_for_stack: conf.too_large_for_stack});
    store.register_early_pass(None, false, box misc_early::MiscEarly);
    store.register_late_pass(None, false, box array_indexing::ArrayIndexing);
    store.register_late_pass(None, false, box panic::Pass);
    store.register_late_pass(None, false, box strings::StringLitAsBytes);
    store.register_late_pass(None, false, box derive::Derive);
    store.register_late_pass(None, false, box types::CharLitAsU8);
    store.register_late_pass(None, false, box write::Pass);
    store.register_late_pass(None, false, box vec::Pass);
    store.register_early_pass(None, false, box non_expressive_names::NonExpressiveNames {
        single_char_binding_names_threshold: conf.single_char_binding_names_threshold,
    });
    store.register_late_pass(None, false, box drop_forget_ref::Pass);
    store.register_late_pass(None, false, box empty_enum::EmptyEnum);
    store.register_late_pass(None, false, box types::AbsurdExtremeComparisons);
    store.register_late_pass(None, false, box types::InvalidUpcastComparisons);
    store.register_late_pass(None, false, box regex::Pass::default());
    store.register_late_pass(None, false, box copies::CopyAndPaste);
    store.register_late_pass(None, false, box format::Pass);
    store.register_early_pass(None, false, box formatting::Formatting);
    store.register_late_pass(None, false, box swap::Swap);
    store.register_early_pass(None, false, box if_not_else::IfNotElse);
    store.register_early_pass(None, false, box else_if_without_else::ElseIfWithoutElse);
    store.register_early_pass(None, false, box int_plus_one::IntPlusOne);
    store.register_late_pass(None, false, box overflow_check_conditional::OverflowCheckConditional);
    store.register_late_pass(None, false, box unused_label::UnusedLabel);
    store.register_late_pass(None, false, box new_without_default::NewWithoutDefault);
    store.register_late_pass(None, false, box blacklisted_name::BlackListedName::new(conf.blacklisted_names));
    store.register_late_pass(None, false, box functions::Functions::new(conf.too_many_arguments_threshold));
    store.register_early_pass(None, false, box doc::Doc::new(conf.doc_valid_idents));
    store.register_late_pass(None, false, box neg_multiply::NegMultiply);
    store.register_early_pass(None, false, box unsafe_removed_from_name::UnsafeNameRemoval);
    store.register_late_pass(None, false, box mem_forget::MemForget);
    store.register_late_pass(None, false, box arithmetic::Arithmetic::default());
    store.register_late_pass(None, false, box assign_ops::AssignOps);
    store.register_late_pass(None, false, box let_if_seq::LetIfSeq);
    store.register_late_pass(None, false, box eval_order_dependence::EvalOrderDependence);
    store.register_late_pass(None, false, box missing_doc::MissingDoc::new());
    store.register_late_pass(None, false, box ok_if_let::Pass);
    store.register_late_pass(None, false, box if_let_redundant_pattern_matching::Pass);
    store.register_late_pass(None, false, box partialeq_ne_impl::Pass);
    store.register_early_pass(None, false, box reference::Pass);
    store.register_early_pass(None, false, box double_parens::DoubleParens);
    store.register_late_pass(None, false, box unused_io_amount::UnusedIoAmount);
    store.register_late_pass(None, false, box large_enum_variant::LargeEnumVariant::new(conf.enum_variant_size_threshold));
    store.register_late_pass(None, false, box explicit_write::Pass);
    store.register_late_pass(None, false, box needless_pass_by_value::NeedlessPassByValue);
    store.register_early_pass(None, false, box literal_representation::LiteralDigitGrouping);
    store.register_early_pass(None, false, box literal_representation::LiteralRepresentation::new(
            conf.literal_representation_threshold
    ));
    store.register_late_pass(None, false, box use_self::UseSelf);
    store.register_late_pass(None, false, box bytecount::ByteCount);
    store.register_late_pass(None, false, box infinite_iter::Pass);
    store.register_late_pass(None, false, box inline_fn_without_body::Pass);
    store.register_late_pass(None, false, box invalid_ref::InvalidRef);
    store.register_late_pass(None, false, box identity_conversion::IdentityConversion::default());
    store.register_late_pass(None, false, box types::ImplicitHasher);
    store.register_early_pass(None, false, box const_static_lifetime::StaticConst);
    store.register_late_pass(None, false, box fallible_impl_from::FallibleImplFrom);
    store.register_late_pass(None, false, box replace_consts::ReplaceConsts);
    store.register_late_pass(None, false, box types::UnitArg);
    store.register_late_pass(None, false, box double_comparison::DoubleComparisonPass);
    store.register_late_pass(None, false, box question_mark::QuestionMarkPass);
    store.register_late_pass(None, false, box suspicious_trait_impl::SuspiciousImpl);
    store.register_late_pass(None, false, box redundant_field_names::RedundantFieldNames);
    store.register_early_pass(None, false, box multiple_crate_versions::Pass);
    store.register_late_pass(None, false, box map_unit_fn::Pass);
    store.register_late_pass(None, false, box infallible_destructuring_match::Pass);

    macro_rules! add_lint_group {
        ($name:expr, $($lint:expr,)*) => (
            store.register_group(Some(sess), false, $name, vec![$(rustc::lint::LintId::of($lint)),*]);
        )
    }

    add_lint_group!("clippy_restriction",
        arithmetic::FLOAT_ARITHMETIC,
        arithmetic::INTEGER_ARITHMETIC,
        array_indexing::INDEXING_SLICING,
        assign_ops::ASSIGN_OPS,
        else_if_without_else::ELSE_IF_WITHOUT_ELSE,
        literal_representation::DECIMAL_LITERAL_REPRESENTATION,
        mem_forget::MEM_FORGET,
        methods::CLONE_ON_REF_PTR,
        methods::OPTION_UNWRAP_USED,
        methods::RESULT_UNWRAP_USED,
        methods::WRONG_PUB_SELF_CONVENTION,
        misc::FLOAT_CMP_CONST,
        missing_doc::MISSING_DOCS_IN_PRIVATE_ITEMS,
        shadow::SHADOW_REUSE,
        shadow::SHADOW_SAME,
        shadow::SHADOW_UNRELATED,
        strings::STRING_ADD,
        write::PRINT_STDOUT,
        write::USE_DEBUG,
    );

    add_lint_group!("clippy_pedantic",
        attrs::INLINE_ALWAYS,
        copies::MATCH_SAME_ARMS,
        derive::EXPL_IMPL_CLONE_ON_COPY,
        doc::DOC_MARKDOWN,
        empty_enum::EMPTY_ENUM,
        enum_glob_use::ENUM_GLOB_USE,
        enum_variants::PUB_ENUM_VARIANT_NAMES,
        enum_variants::STUTTER,
        if_not_else::IF_NOT_ELSE,
        infinite_iter::MAYBE_INFINITE_ITER,
        items_after_statements::ITEMS_AFTER_STATEMENTS,
        matches::SINGLE_MATCH_ELSE,
        methods::FILTER_MAP,
        methods::OPTION_MAP_UNWRAP_OR,
        methods::OPTION_MAP_UNWRAP_OR_ELSE,
        methods::RESULT_MAP_UNWRAP_OR_ELSE,
        misc::USED_UNDERSCORE_BINDING,
        misc_early::UNSEPARATED_LITERAL_SUFFIX,
        mut_mut::MUT_MUT,
        needless_continue::NEEDLESS_CONTINUE,
        non_expressive_names::SIMILAR_NAMES,
        replace_consts::REPLACE_CONSTS,
        strings::STRING_ADD_ASSIGN,
        types::CAST_POSSIBLE_TRUNCATION,
        types::CAST_POSSIBLE_WRAP,
        types::CAST_PRECISION_LOSS,
        types::CAST_SIGN_LOSS,
        types::INVALID_UPCAST_COMPARISONS,
        types::LINKEDLIST,
        unicode::NON_ASCII_LITERAL,
        unicode::UNICODE_NOT_NFC,
        use_self::USE_SELF,
    );

    add_lint_group!("clippy_internal",
        utils::internal_lints::CLIPPY_LINTS_INTERNAL,
        utils::internal_lints::LINT_WITHOUT_LINT_PASS,
    );

    add_lint_group!("clippy",
        approx_const::APPROX_CONSTANT,
        array_indexing::OUT_OF_BOUNDS_INDEXING,
        assign_ops::ASSIGN_OP_PATTERN,
        assign_ops::MISREFACTORED_ASSIGN_OP,
        attrs::DEPRECATED_SEMVER,
        attrs::USELESS_ATTRIBUTE,
        bit_mask::BAD_BIT_MASK,
        bit_mask::INEFFECTIVE_BIT_MASK,
        bit_mask::VERBOSE_BIT_MASK,
        blacklisted_name::BLACKLISTED_NAME,
        block_in_if_condition::BLOCK_IN_IF_CONDITION_EXPR,
        block_in_if_condition::BLOCK_IN_IF_CONDITION_STMT,
        booleans::LOGIC_BUG,
        booleans::NONMINIMAL_BOOL,
        bytecount::NAIVE_BYTECOUNT,
        collapsible_if::COLLAPSIBLE_IF,
        const_static_lifetime::CONST_STATIC_LIFETIME,
        copies::IF_SAME_THEN_ELSE,
        copies::IFS_SAME_COND,
        cyclomatic_complexity::CYCLOMATIC_COMPLEXITY,
        derive::DERIVE_HASH_XOR_EQ,
        double_comparison::DOUBLE_COMPARISONS,
        double_parens::DOUBLE_PARENS,
        drop_forget_ref::DROP_COPY,
        drop_forget_ref::DROP_REF,
        drop_forget_ref::FORGET_COPY,
        drop_forget_ref::FORGET_REF,
        entry::MAP_ENTRY,
        enum_clike::ENUM_CLIKE_UNPORTABLE_VARIANT,
        enum_variants::ENUM_VARIANT_NAMES,
        enum_variants::MODULE_INCEPTION,
        eq_op::EQ_OP,
        eq_op::OP_REF,
        erasing_op::ERASING_OP,
        escape::BOXED_LOCAL,
        eta_reduction::REDUNDANT_CLOSURE,
        eval_order_dependence::DIVERGING_SUB_EXPRESSION,
        eval_order_dependence::EVAL_ORDER_DEPENDENCE,
        excessive_precision::EXCESSIVE_PRECISION,
        explicit_write::EXPLICIT_WRITE,
        format::USELESS_FORMAT,
        formatting::POSSIBLE_MISSING_COMMA,
        formatting::SUSPICIOUS_ASSIGNMENT_FORMATTING,
        formatting::SUSPICIOUS_ELSE_FORMATTING,
        functions::NOT_UNSAFE_PTR_ARG_DEREF,
        functions::TOO_MANY_ARGUMENTS,
        identity_conversion::IDENTITY_CONVERSION,
        identity_op::IDENTITY_OP,
        if_let_redundant_pattern_matching::IF_LET_REDUNDANT_PATTERN_MATCHING,
        infallible_destructuring_match::INFALLIBLE_DESTRUCTURING_MATCH,
        infinite_iter::INFINITE_ITER,
        inline_fn_without_body::INLINE_FN_WITHOUT_BODY,
        int_plus_one::INT_PLUS_ONE,
        invalid_ref::INVALID_REF,
        large_enum_variant::LARGE_ENUM_VARIANT,
        len_zero::LEN_WITHOUT_IS_EMPTY,
        len_zero::LEN_ZERO,
        let_if_seq::USELESS_LET_IF_SEQ,
        lifetimes::NEEDLESS_LIFETIMES,
        lifetimes::UNUSED_LIFETIMES,
        literal_representation::INCONSISTENT_DIGIT_GROUPING,
        literal_representation::LARGE_DIGIT_GROUPS,
        literal_representation::UNREADABLE_LITERAL,
        loops::EMPTY_LOOP,
        loops::EXPLICIT_COUNTER_LOOP,
        loops::EXPLICIT_INTO_ITER_LOOP,
        loops::EXPLICIT_ITER_LOOP,
        loops::FOR_KV_MAP,
        loops::FOR_LOOP_OVER_OPTION,
        loops::FOR_LOOP_OVER_RESULT,
        loops::ITER_NEXT_LOOP,
        loops::MANUAL_MEMCPY,
        loops::MUT_RANGE_BOUND,
        loops::NEEDLESS_RANGE_LOOP,
        loops::NEVER_LOOP,
        loops::REVERSE_RANGE_LOOP,
        loops::UNUSED_COLLECT,
        loops::WHILE_IMMUTABLE_CONDITION,
        loops::WHILE_LET_LOOP,
        loops::WHILE_LET_ON_ITERATOR,
        map_clone::MAP_CLONE,
        map_unit_fn::OPTION_MAP_UNIT_FN,
        map_unit_fn::RESULT_MAP_UNIT_FN,
        matches::MATCH_AS_REF,
        matches::MATCH_BOOL,
        matches::MATCH_OVERLAPPING_ARM,
        matches::MATCH_REF_PATS,
        matches::MATCH_WILD_ERR_ARM,
        matches::SINGLE_MATCH,
        methods::CHARS_LAST_CMP,
        methods::CHARS_NEXT_CMP,
        methods::CLONE_DOUBLE_REF,
        methods::CLONE_ON_COPY,
        methods::FILTER_NEXT,
        methods::GET_UNWRAP,
        methods::ITER_CLONED_COLLECT,
        methods::ITER_NTH,
        methods::ITER_SKIP_NEXT,
        methods::NEW_RET_NO_SELF,
        methods::OK_EXPECT,
        methods::OPTION_MAP_OR_NONE,
        methods::OR_FUN_CALL,
        methods::SEARCH_IS_SOME,
        methods::SHOULD_IMPLEMENT_TRAIT,
        methods::SINGLE_CHAR_PATTERN,
        methods::STRING_EXTEND_CHARS,
        methods::TEMPORARY_CSTRING_AS_PTR,
        methods::UNNECESSARY_FOLD,
        methods::USELESS_ASREF,
        methods::WRONG_SELF_CONVENTION,
        minmax::MIN_MAX,
        misc::CMP_NAN,
        misc::CMP_OWNED,
        misc::FLOAT_CMP,
        misc::MODULO_ONE,
        misc::REDUNDANT_PATTERN,
        misc::SHORT_CIRCUIT_STATEMENT,
        misc::TOPLEVEL_REF_ARG,
        misc::ZERO_PTR,
        misc_early::BUILTIN_TYPE_SHADOW,
        misc_early::DOUBLE_NEG,
        misc_early::DUPLICATE_UNDERSCORE_ARGUMENT,
        misc_early::MIXED_CASE_HEX_LITERALS,
        misc_early::REDUNDANT_CLOSURE_CALL,
        misc_early::UNNEEDED_FIELD_PATTERN,
        misc_early::ZERO_PREFIXED_LITERAL,
        mut_reference::UNNECESSARY_MUT_PASSED,
        mutex_atomic::MUTEX_ATOMIC,
        needless_bool::BOOL_COMPARISON,
        needless_bool::NEEDLESS_BOOL,
        needless_borrowed_ref::NEEDLESS_BORROWED_REFERENCE,
        needless_pass_by_value::NEEDLESS_PASS_BY_VALUE,
        needless_update::NEEDLESS_UPDATE,
        neg_multiply::NEG_MULTIPLY,
        new_without_default::NEW_WITHOUT_DEFAULT,
        new_without_default::NEW_WITHOUT_DEFAULT_DERIVE,
        no_effect::NO_EFFECT,
        no_effect::UNNECESSARY_OPERATION,
        non_expressive_names::JUST_UNDERSCORES_AND_DIGITS,
        non_expressive_names::MANY_SINGLE_CHAR_NAMES,
        ok_if_let::IF_LET_SOME_RESULT,
        open_options::NONSENSICAL_OPEN_OPTIONS,
        overflow_check_conditional::OVERFLOW_CHECK_CONDITIONAL,
        panic::PANIC_PARAMS,
        partialeq_ne_impl::PARTIALEQ_NE_IMPL,
        precedence::PRECEDENCE,
        ptr::CMP_NULL,
        ptr::MUT_FROM_REF,
        ptr::PTR_ARG,
        question_mark::QUESTION_MARK,
        ranges::ITERATOR_STEP_BY_ZERO,
        ranges::RANGE_MINUS_ONE,
        ranges::RANGE_ZIP_WITH_LEN,
        redundant_field_names::REDUNDANT_FIELD_NAMES,
        reference::DEREF_ADDROF,
        regex::INVALID_REGEX,
        regex::REGEX_MACRO,
        regex::TRIVIAL_REGEX,
        returns::LET_AND_RETURN,
        returns::NEEDLESS_RETURN,
        serde_api::SERDE_API_MISUSE,
        strings::STRING_LIT_AS_BYTES,
        suspicious_trait_impl::SUSPICIOUS_ARITHMETIC_IMPL,
        suspicious_trait_impl::SUSPICIOUS_OP_ASSIGN_IMPL,
        swap::ALMOST_SWAPPED,
        swap::MANUAL_SWAP,
        temporary_assignment::TEMPORARY_ASSIGNMENT,
        transmute::CROSSPOINTER_TRANSMUTE,
        transmute::TRANSMUTE_BYTES_TO_STR,
        transmute::TRANSMUTE_INT_TO_BOOL,
        transmute::TRANSMUTE_INT_TO_CHAR,
        transmute::TRANSMUTE_INT_TO_FLOAT,
        transmute::TRANSMUTE_PTR_TO_PTR,
        transmute::TRANSMUTE_PTR_TO_REF,
        transmute::USELESS_TRANSMUTE,
        transmute::WRONG_TRANSMUTE,
        types::ABSURD_EXTREME_COMPARISONS,
        types::BORROWED_BOX,
        types::BOX_VEC,
        types::CAST_LOSSLESS,
        types::CAST_PTR_ALIGNMENT,
        types::CHAR_LIT_AS_U8,
        types::IMPLICIT_HASHER,
        types::LET_UNIT_VALUE,
        types::OPTION_OPTION,
        types::TYPE_COMPLEXITY,
        types::UNIT_ARG,
        types::UNIT_CMP,
        types::UNNECESSARY_CAST,
        unicode::ZERO_WIDTH_SPACE,
        unsafe_removed_from_name::UNSAFE_REMOVED_FROM_NAME,
        unused_io_amount::UNUSED_IO_AMOUNT,
        unused_label::UNUSED_LABEL,
        vec::USELESS_VEC,
        write::PRINT_LITERAL,
        write::PRINT_WITH_NEWLINE,
        write::PRINTLN_EMPTY_STRING,
        write::WRITE_LITERAL,
        write::WRITE_WITH_NEWLINE,
        write::WRITELN_EMPTY_STRING,
        zero_div_zero::ZERO_DIVIDED_BY_ZERO,
    );

    add_lint_group!("clippy_style",
        assign_ops::ASSIGN_OP_PATTERN,
        bit_mask::VERBOSE_BIT_MASK,
        blacklisted_name::BLACKLISTED_NAME,
        block_in_if_condition::BLOCK_IN_IF_CONDITION_EXPR,
        block_in_if_condition::BLOCK_IN_IF_CONDITION_STMT,
        collapsible_if::COLLAPSIBLE_IF,
        const_static_lifetime::CONST_STATIC_LIFETIME,
        enum_variants::ENUM_VARIANT_NAMES,
        enum_variants::MODULE_INCEPTION,
        eq_op::OP_REF,
        eta_reduction::REDUNDANT_CLOSURE,
        excessive_precision::EXCESSIVE_PRECISION,
        formatting::SUSPICIOUS_ASSIGNMENT_FORMATTING,
        formatting::SUSPICIOUS_ELSE_FORMATTING,
        if_let_redundant_pattern_matching::IF_LET_REDUNDANT_PATTERN_MATCHING,
        infallible_destructuring_match::INFALLIBLE_DESTRUCTURING_MATCH,
        len_zero::LEN_WITHOUT_IS_EMPTY,
        len_zero::LEN_ZERO,
        let_if_seq::USELESS_LET_IF_SEQ,
        literal_representation::INCONSISTENT_DIGIT_GROUPING,
        literal_representation::LARGE_DIGIT_GROUPS,
        literal_representation::UNREADABLE_LITERAL,
        loops::EMPTY_LOOP,
        loops::EXPLICIT_INTO_ITER_LOOP,
        loops::EXPLICIT_ITER_LOOP,
        loops::FOR_KV_MAP,
        loops::NEEDLESS_RANGE_LOOP,
        loops::WHILE_LET_ON_ITERATOR,
        map_clone::MAP_CLONE,
        matches::MATCH_BOOL,
        matches::MATCH_OVERLAPPING_ARM,
        matches::MATCH_REF_PATS,
        matches::MATCH_WILD_ERR_ARM,
        matches::SINGLE_MATCH,
        methods::CHARS_LAST_CMP,
        methods::GET_UNWRAP,
        methods::ITER_CLONED_COLLECT,
        methods::ITER_SKIP_NEXT,
        methods::NEW_RET_NO_SELF,
        methods::OK_EXPECT,
        methods::OPTION_MAP_OR_NONE,
        methods::SHOULD_IMPLEMENT_TRAIT,
        methods::STRING_EXTEND_CHARS,
        methods::UNNECESSARY_FOLD,
        methods::WRONG_SELF_CONVENTION,
        misc::REDUNDANT_PATTERN,
        misc::TOPLEVEL_REF_ARG,
        misc::ZERO_PTR,
        misc_early::BUILTIN_TYPE_SHADOW,
        misc_early::DOUBLE_NEG,
        misc_early::DUPLICATE_UNDERSCORE_ARGUMENT,
        misc_early::MIXED_CASE_HEX_LITERALS,
        misc_early::UNNEEDED_FIELD_PATTERN,
        mut_reference::UNNECESSARY_MUT_PASSED,
        needless_pass_by_value::NEEDLESS_PASS_BY_VALUE,
        neg_multiply::NEG_MULTIPLY,
        new_without_default::NEW_WITHOUT_DEFAULT,
        new_without_default::NEW_WITHOUT_DEFAULT_DERIVE,
        non_expressive_names::JUST_UNDERSCORES_AND_DIGITS,
        non_expressive_names::MANY_SINGLE_CHAR_NAMES,
        ok_if_let::IF_LET_SOME_RESULT,
        panic::PANIC_PARAMS,
        ptr::CMP_NULL,
        ptr::PTR_ARG,
        question_mark::QUESTION_MARK,
        ranges::RANGE_MINUS_ONE,
        redundant_field_names::REDUNDANT_FIELD_NAMES,
        regex::REGEX_MACRO,
        regex::TRIVIAL_REGEX,
        returns::LET_AND_RETURN,
        returns::NEEDLESS_RETURN,
        strings::STRING_LIT_AS_BYTES,
        types::IMPLICIT_HASHER,
        types::LET_UNIT_VALUE,
        unsafe_removed_from_name::UNSAFE_REMOVED_FROM_NAME,
        write::PRINT_LITERAL,
        write::PRINT_WITH_NEWLINE,
        write::PRINTLN_EMPTY_STRING,
        write::WRITE_LITERAL,
        write::WRITE_WITH_NEWLINE,
        write::WRITELN_EMPTY_STRING,
    );

    add_lint_group!("clippy_complexity",
        assign_ops::MISREFACTORED_ASSIGN_OP,
        booleans::NONMINIMAL_BOOL,
        cyclomatic_complexity::CYCLOMATIC_COMPLEXITY,
        double_comparison::DOUBLE_COMPARISONS,
        double_parens::DOUBLE_PARENS,
        eval_order_dependence::DIVERGING_SUB_EXPRESSION,
        eval_order_dependence::EVAL_ORDER_DEPENDENCE,
        explicit_write::EXPLICIT_WRITE,
        format::USELESS_FORMAT,
        functions::TOO_MANY_ARGUMENTS,
        identity_conversion::IDENTITY_CONVERSION,
        identity_op::IDENTITY_OP,
        int_plus_one::INT_PLUS_ONE,
        lifetimes::NEEDLESS_LIFETIMES,
        lifetimes::UNUSED_LIFETIMES,
        loops::EXPLICIT_COUNTER_LOOP,
        loops::MUT_RANGE_BOUND,
        loops::WHILE_LET_LOOP,
        map_unit_fn::OPTION_MAP_UNIT_FN,
        map_unit_fn::RESULT_MAP_UNIT_FN,
        matches::MATCH_AS_REF,
        methods::CHARS_NEXT_CMP,
        methods::CLONE_ON_COPY,
        methods::FILTER_NEXT,
        methods::SEARCH_IS_SOME,
        methods::USELESS_ASREF,
        misc::SHORT_CIRCUIT_STATEMENT,
        misc_early::REDUNDANT_CLOSURE_CALL,
        misc_early::ZERO_PREFIXED_LITERAL,
        needless_bool::BOOL_COMPARISON,
        needless_bool::NEEDLESS_BOOL,
        needless_borrowed_ref::NEEDLESS_BORROWED_REFERENCE,
        needless_update::NEEDLESS_UPDATE,
        no_effect::NO_EFFECT,
        no_effect::UNNECESSARY_OPERATION,
        overflow_check_conditional::OVERFLOW_CHECK_CONDITIONAL,
        partialeq_ne_impl::PARTIALEQ_NE_IMPL,
        precedence::PRECEDENCE,
        ranges::RANGE_ZIP_WITH_LEN,
        reference::DEREF_ADDROF,
        swap::MANUAL_SWAP,
        temporary_assignment::TEMPORARY_ASSIGNMENT,
        transmute::CROSSPOINTER_TRANSMUTE,
        transmute::TRANSMUTE_BYTES_TO_STR,
        transmute::TRANSMUTE_INT_TO_BOOL,
        transmute::TRANSMUTE_INT_TO_CHAR,
        transmute::TRANSMUTE_INT_TO_FLOAT,
        transmute::TRANSMUTE_PTR_TO_PTR,
        transmute::TRANSMUTE_PTR_TO_REF,
        transmute::USELESS_TRANSMUTE,
        types::BORROWED_BOX,
        types::CAST_LOSSLESS,
        types::CHAR_LIT_AS_U8,
        types::OPTION_OPTION,
        types::TYPE_COMPLEXITY,
        types::UNIT_ARG,
        types::UNNECESSARY_CAST,
        unused_label::UNUSED_LABEL,
        zero_div_zero::ZERO_DIVIDED_BY_ZERO,
    );

    add_lint_group!("clippy_correctness",
        approx_const::APPROX_CONSTANT,
        array_indexing::OUT_OF_BOUNDS_INDEXING,
        attrs::DEPRECATED_SEMVER,
        attrs::USELESS_ATTRIBUTE,
        bit_mask::BAD_BIT_MASK,
        bit_mask::INEFFECTIVE_BIT_MASK,
        booleans::LOGIC_BUG,
        copies::IF_SAME_THEN_ELSE,
        copies::IFS_SAME_COND,
        derive::DERIVE_HASH_XOR_EQ,
        drop_forget_ref::DROP_COPY,
        drop_forget_ref::DROP_REF,
        drop_forget_ref::FORGET_COPY,
        drop_forget_ref::FORGET_REF,
        enum_clike::ENUM_CLIKE_UNPORTABLE_VARIANT,
        eq_op::EQ_OP,
        erasing_op::ERASING_OP,
        formatting::POSSIBLE_MISSING_COMMA,
        functions::NOT_UNSAFE_PTR_ARG_DEREF,
        infinite_iter::INFINITE_ITER,
        inline_fn_without_body::INLINE_FN_WITHOUT_BODY,
        invalid_ref::INVALID_REF,
        loops::FOR_LOOP_OVER_OPTION,
        loops::FOR_LOOP_OVER_RESULT,
        loops::ITER_NEXT_LOOP,
        loops::NEVER_LOOP,
        loops::REVERSE_RANGE_LOOP,
        loops::WHILE_IMMUTABLE_CONDITION,
        methods::CLONE_DOUBLE_REF,
        methods::TEMPORARY_CSTRING_AS_PTR,
        minmax::MIN_MAX,
        misc::CMP_NAN,
        misc::FLOAT_CMP,
        misc::MODULO_ONE,
        open_options::NONSENSICAL_OPEN_OPTIONS,
        ptr::MUT_FROM_REF,
        ranges::ITERATOR_STEP_BY_ZERO,
        regex::INVALID_REGEX,
        serde_api::SERDE_API_MISUSE,
        suspicious_trait_impl::SUSPICIOUS_ARITHMETIC_IMPL,
        suspicious_trait_impl::SUSPICIOUS_OP_ASSIGN_IMPL,
        swap::ALMOST_SWAPPED,
        transmute::WRONG_TRANSMUTE,
        types::ABSURD_EXTREME_COMPARISONS,
        types::CAST_PTR_ALIGNMENT,
        types::UNIT_CMP,
        unicode::ZERO_WIDTH_SPACE,
        unused_io_amount::UNUSED_IO_AMOUNT,
    );

    add_lint_group!("clippy_perf",
        bytecount::NAIVE_BYTECOUNT,
        entry::MAP_ENTRY,
        escape::BOXED_LOCAL,
        large_enum_variant::LARGE_ENUM_VARIANT,
        loops::MANUAL_MEMCPY,
        loops::UNUSED_COLLECT,
        methods::ITER_NTH,
        methods::OR_FUN_CALL,
        methods::SINGLE_CHAR_PATTERN,
        misc::CMP_OWNED,
        mutex_atomic::MUTEX_ATOMIC,
        types::BOX_VEC,
        vec::USELESS_VEC,
    );

    add_lint_group!("clippy_cargo",
        multiple_crate_versions::MULTIPLE_CRATE_VERSIONS,
    );

    add_lint_group!("clippy_nursery",
        attrs::EMPTY_LINE_AFTER_OUTER_ATTR,
        fallible_impl_from::FALLIBLE_IMPL_FROM,
        mutex_atomic::MUTEX_INTEGER,
        needless_borrow::NEEDLESS_BORROW,
        ranges::RANGE_PLUS_ONE,
    );
}

// only exists to let the dogfood integration test works.
// Don't run clippy as an executable directly
#[allow(dead_code, print_stdout)]
fn main() {
    panic!("Please use the cargo-clippy executable");
}
