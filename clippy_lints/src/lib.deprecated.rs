// This file was generated by `cargo dev update_lints`.
// Use that command to update this file and do not edit by hand.
// Manual edits will be overwritten.

{
    store.register_removed(
        "clippy::should_assert_eq",
        "`assert!()` will be more flexible with RFC 2011",
    );
    store.register_removed(
        "clippy::extend_from_slice",
        "`.extend_from_slice(_)` is a faster way to extend a Vec by a slice",
    );
    store.register_removed(
        "clippy::range_step_by_zero",
        "`iterator.step_by(0)` panics nowadays",
    );
    store.register_removed(
        "clippy::unstable_as_slice",
        "`Vec::as_slice` has been stabilized in 1.7",
    );
    store.register_removed(
        "clippy::unstable_as_mut_slice",
        "`Vec::as_mut_slice` has been stabilized in 1.7",
    );
    store.register_removed(
        "clippy::misaligned_transmute",
        "this lint has been split into cast_ptr_alignment and transmute_ptr_to_ptr",
    );
    store.register_removed(
        "clippy::assign_ops",
        "using compound assignment operators (e.g., `+=`) is harmless",
    );
    store.register_removed(
        "clippy::if_let_redundant_pattern_matching",
        "this lint has been changed to redundant_pattern_matching",
    );
    store.register_removed(
        "clippy::unsafe_vector_initialization",
        "the replacement suggested by this lint had substantially different behavior",
    );
    store.register_removed(
        "clippy::unused_collect",
        "`collect` has been marked as #[must_use] in rustc and that covers all cases of this lint",
    );
    store.register_removed(
        "clippy::replace_consts",
        "associated-constants `MIN`/`MAX` of integers are preferred to `{min,max}_value()` and module constants",
    );
    store.register_removed(
        "clippy::regex_macro",
        "the regex! macro has been removed from the regex crate in 2018",
    );
    store.register_removed(
        "clippy::find_map",
        "this lint has been replaced by `manual_find_map`, a more specific lint",
    );
    store.register_removed(
        "clippy::filter_map",
        "this lint has been replaced by `manual_filter_map`, a more specific lint",
    );
    store.register_removed(
        "clippy::pub_enum_variant_names",
        "set the `avoid-breaking-exported-api` config option to `false` to enable the `enum_variant_names` lint for public items",
    );
    store.register_removed(
        "clippy::wrong_pub_self_convention",
        "set the `avoid-breaking-exported-api` config option to `false` to enable the `wrong_self_convention` lint for public items",
    );
    store.register_removed(
        "clippy::maybe_misused_cfg",
        "this lint has been replaced by `unexpected_cfgs`",
    );
    store.register_removed(
        "clippy::mismatched_target_os",
        "this lint has been replaced by `unexpected_cfgs`",
    );
    store.register_removed(
        "clippy::float_cmp_const",
        "`float_cmp` handles this via config options",
    );
}
