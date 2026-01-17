// This file is managed by `cargo dev rename_lint` and `cargo dev deprecate_lint`.
// Prefer to use those when possible.

#[derive(Copy, Clone, Debug)]
pub struct Deprecation {
    pub name: &'static str,
    pub reason: &'static str,
    pub version: &'static str,
}

macro_rules! deprecated {
    ($(
        #[clippy::version = $version:literal]
        ($name:literal, $reason:literal),
    )*) => {
        &[$(Deprecation {
            name: $name,
            reason: $reason,
            version: $version,
        }),*]
    };
}

#[rustfmt::skip]
pub const DEPRECATED: &[Deprecation] = deprecated![
    #[clippy::version = "1.30.0"]
    ("clippy::assign_ops", "compound operators are harmless and linting on them is not in scope for clippy"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::extend_from_slice", "`Vec::extend_from_slice` is no longer faster than `Vec::extend` due to specialization"),
    #[clippy::version = "1.88.0"]
    ("clippy::match_on_vec_items", "`clippy::indexing_slicing` covers indexing and slicing on `Vec<_>`"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::misaligned_transmute", "split into `clippy::cast_ptr_alignment` and `clippy::transmute_ptr_to_ptr`"),
    #[clippy::version = "1.87.0"]
    ("clippy::option_map_or_err_ok", "`clippy::manual_ok_or` covers this case"),
    #[clippy::version = "1.54.0"]
    ("clippy::pub_enum_variant_names", "`clippy::enum_variant_names` now covers this case via the `avoid-breaking-exported-api` config"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::range_step_by_zero", "`Iterator::step_by(0)` now panics and is no longer an infinite iterator"),
    #[clippy::version = "1.47.0"]
    ("clippy::regex_macro", "the `regex!` macro was removed from the regex crate in 2018"),
    #[clippy::version = "1.44.0"]
    ("clippy::replace_consts", "`min_value` and `max_value` are now deprecated"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::should_assert_eq", "`assert!(a == b)` can now print the values the same way `assert_eq!(a, b) can"),
    #[clippy::version = "1.91.0"]
    ("clippy::string_to_string", "`clippy::implicit_clone` covers those cases"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::unsafe_vector_initialization", "the suggested alternative could be substantially slower"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::unstable_as_mut_slice", "`Vec::as_mut_slice` is now stable"),
    #[clippy::version = "pre 1.29.0"]
    ("clippy::unstable_as_slice", "`Vec::as_slice` is now stable"),
    #[clippy::version = "1.39.0"]
    ("clippy::unused_collect", "`Iterator::collect` is now marked as `#[must_use]`"),
    #[clippy::version = "1.54.0"]
    ("clippy::wrong_pub_self_convention", "`clippy::wrong_self_convention` now covers this case via the `avoid-breaking-exported-api` config"),
];

#[rustfmt::skip]
pub const RENAMED: &[(&str, &str)] = &[
    ("clippy::almost_complete_letter_range", "clippy::almost_complete_range"),
    ("clippy::blacklisted_name", "clippy::disallowed_names"),
    ("clippy::block_in_if_condition_expr", "clippy::blocks_in_conditions"),
    ("clippy::block_in_if_condition_stmt", "clippy::blocks_in_conditions"),
    ("clippy::blocks_in_if_conditions", "clippy::blocks_in_conditions"),
    ("clippy::box_vec", "clippy::box_collection"),
    ("clippy::cast_ref_to_mut", "invalid_reference_casting"),
    ("clippy::clone_double_ref", "suspicious_double_ref_op"),
    ("clippy::cmp_nan", "invalid_nan_comparisons"),
    ("clippy::const_static_lifetime", "clippy::redundant_static_lifetimes"),
    ("clippy::cyclomatic_complexity", "clippy::cognitive_complexity"),
    ("clippy::derive_hash_xor_eq", "clippy::derived_hash_with_manual_eq"),
    ("clippy::disallowed_method", "clippy::disallowed_methods"),
    ("clippy::disallowed_type", "clippy::disallowed_types"),
    ("clippy::double_neg", "double_negations"),
    ("clippy::drop_bounds", "drop_bounds"),
    ("clippy::drop_copy", "dropping_copy_types"),
    ("clippy::drop_ref", "dropping_references"),
    ("clippy::empty_enum", "clippy::empty_enums"),
    ("clippy::eval_order_dependence", "clippy::mixed_read_write_in_expression"),
    ("clippy::filter_map", "clippy::manual_filter_map"),
    ("clippy::find_map", "clippy::manual_find_map"),
    ("clippy::fn_address_comparisons", "unpredictable_function_pointer_comparisons"),
    ("clippy::fn_null_check", "useless_ptr_null_checks"),
    ("clippy::for_loop_over_option", "for_loops_over_fallibles"),
    ("clippy::for_loop_over_result", "for_loops_over_fallibles"),
    ("clippy::for_loops_over_fallibles", "for_loops_over_fallibles"),
    ("clippy::forget_copy", "forgetting_copy_types"),
    ("clippy::forget_ref", "forgetting_references"),
    ("clippy::identity_conversion", "clippy::useless_conversion"),
    ("clippy::if_let_redundant_pattern_matching", "clippy::redundant_pattern_matching"),
    ("clippy::if_let_some_result", "clippy::match_result_ok"),
    ("clippy::incorrect_clone_impl_on_copy_type", "clippy::non_canonical_clone_impl"),
    ("clippy::incorrect_partial_ord_impl_on_ord_type", "clippy::non_canonical_partial_ord_impl"),
    ("clippy::integer_arithmetic", "clippy::arithmetic_side_effects"),
    ("clippy::into_iter_on_array", "array_into_iter"),
    ("clippy::invalid_atomic_ordering", "invalid_atomic_ordering"),
    ("clippy::invalid_null_ptr_usage", "invalid_null_arguments"),
    ("clippy::invalid_ref", "invalid_value"),
    ("clippy::invalid_utf8_in_unchecked", "invalid_from_utf8_unchecked"),
    ("clippy::let_underscore_drop", "let_underscore_drop"),
    ("clippy::logic_bug", "clippy::overly_complex_bool_expr"),
    ("clippy::maybe_misused_cfg", "unexpected_cfgs"),
    ("clippy::mem_discriminant_non_enum", "enum_intrinsics_non_enums"),
    ("clippy::mismatched_target_os", "unexpected_cfgs"),
    ("clippy::needless_if", "clippy::needless_ifs"),
    ("clippy::new_without_default_derive", "clippy::new_without_default"),
    ("clippy::option_and_then_some", "clippy::bind_instead_of_map"),
    ("clippy::option_expect_used", "clippy::expect_used"),
    ("clippy::option_map_unwrap_or", "clippy::map_unwrap_or"),
    ("clippy::option_map_unwrap_or_else", "clippy::map_unwrap_or"),
    ("clippy::option_unwrap_used", "clippy::unwrap_used"),
    ("clippy::overflow_check_conditional", "clippy::panicking_overflow_checks"),
    ("clippy::panic_params", "non_fmt_panics"),
    ("clippy::positional_named_format_parameters", "named_arguments_used_positionally"),
    ("clippy::ref_in_deref", "clippy::needless_borrow"),
    ("clippy::result_expect_used", "clippy::expect_used"),
    ("clippy::result_map_unwrap_or_else", "clippy::map_unwrap_or"),
    ("clippy::result_unwrap_used", "clippy::unwrap_used"),
    ("clippy::reverse_range_loop", "clippy::reversed_empty_ranges"),
    ("clippy::single_char_push_str", "clippy::single_char_add_str"),
    ("clippy::stutter", "clippy::module_name_repetitions"),
    ("clippy::temporary_cstring_as_ptr", "dangling_pointers_from_temporaries"),
    ("clippy::thread_local_initializer_can_be_made_const", "clippy::missing_const_for_thread_local"),
    ("clippy::to_string_in_display", "clippy::recursive_format_impl"),
    ("clippy::transmute_float_to_int", "unnecessary_transmutes"),
    ("clippy::transmute_int_to_char", "unnecessary_transmutes"),
    ("clippy::transmute_int_to_float", "unnecessary_transmutes"),
    ("clippy::transmute_num_to_bytes", "unnecessary_transmutes"),
    ("clippy::unchecked_duration_subtraction", "clippy::unchecked_time_subtraction"),
    ("clippy::undropped_manually_drops", "undropped_manually_drops"),
    ("clippy::unknown_clippy_lints", "unknown_lints"),
    ("clippy::unused_label", "unused_labels"),
    ("clippy::unwrap_or_else_default", "clippy::unwrap_or_default"),
    ("clippy::vtable_address_comparisons", "ambiguous_wide_pointer_comparisons"),
    ("clippy::zero_width_space", "clippy::invisible_characters"),
];
