# Lint subgroups

Clippy groups its lints into [9 primary categories](lints.md),
two of which are allow-by-default (pedantic and restriction).

One downside of having such few but broad categories for allow-by-default lints
is that it significantly decreases discoverability, as `restriction` often acts as a blanket category
for any lint that most users likely would not want enforced on their codebase.

This page should help with that, by defining more granular, unofficial lint (sub)groups.
For example, some people might not be interested in all the style-related `pedantic` lints,
but *are* interested in the `perf`-related ones, so these lints
can additionally be added to the [`perf_pedantic`](#perf_pedantic) subgroup.

<!--
NOTE: Do not edit the contents in between lint-subgroup-start and lint-subgroup-end manually.
Instead, change the `declare_clippy_lint!` macro invocation for the particular lint
to include it in (or remove it from) a subgroup and re-run `cargo dev update_lints`.
The descriptions however are fine to edit.
-->

## `perf_pedantic`

These are `pedantic` lints that look for code patterns that could be expressed in a more efficient way.
These would be candidates for the `perf` category, however suggestions made by them can also sometimes hurt readability
and obfuscate the meaning, so occasional `#[allow]`s are expected to be used.

<!-- lint-subgroup-start: perf_pedantic -->
Lints: `assigning_clones`, `inefficient_to_string`, `naive_bytecount`, `needless_bitwise_bool`, `trivially_copy_pass_by_ref`, `unnecessary_box_returns`, `unnecessary_join`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::assigning_clones,
    clippy::inefficient_to_string,
    clippy::naive_bytecount,
    clippy::needless_bitwise_bool,
    clippy::trivially_copy_pass_by_ref,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_join
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
assigning_clones = "warn"
inefficient_to_string = "warn"
naive_bytecount = "warn"
needless_bitwise_bool = "warn"
trivially_copy_pass_by_ref = "warn"
unnecessary_box_returns = "warn"
unnecessary_join = "warn"
```
</details>
<!-- lint-subgroup-end -->


## `perf_restriction`

These are `restriction` lints that can improve the performance of code, but are very specific
and sometimes *significantly* hurt readability with very little gain in the usual case.
These should ideally only be applied to specific functions or modules that were profiled
and where it is very clear that any performance gain matters.

As always (but especially here), you should double-check that applying these actually helps
and that any performance wins are worth the introduced complexity.

<!-- lint-subgroup-start: perf_restriction -->
Lints: `format_push_string`, `missing_asserts_for_indexing`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::format_push_string,
    clippy::missing_asserts_for_indexing
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
format_push_string = "warn"
missing_asserts_for_indexing = "warn"
```
</details>
<!-- lint-subgroup-end -->

## `perf_nursery`

These are `nursery` lints that either were previously in the `perf` category or are intended to be in `perf`
but have too many false positives.
Some of them may also be simply wrong in certain situations and end up slower,
so you should make sure to read the description to learn about possible edge cases.

<!-- lint-subgroup-start: perf_nursery -->
Lints: `iter_with_drain`, `mutex_integer`, `needless_collect`, `or_fun_call`, `redundant_clone`, `significant_drop_tightening`, `trivial_regex`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::iter_with_drain,
    clippy::mutex_integer,
    clippy::needless_collect,
    clippy::or_fun_call,
    clippy::redundant_clone,
    clippy::significant_drop_tightening,
    clippy::trivial_regex
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
iter_with_drain = "warn"
mutex_integer = "warn"
needless_collect = "warn"
or_fun_call = "warn"
redundant_clone = "warn"
significant_drop_tightening = "warn"
trivial_regex = "warn"
```
</details>
<!-- lint-subgroup-end -->

## `panic`

These are `restriction` lints that look for patterns that can introduce panics.

Usually panics are not something that one should want to avoid and most of the time panicking is perfectly valid
(hence why these lints are in the `restriction` category),
but users may want to forbid any use of panicky functions altogether in specific contexts.

One use case could be to annotate `GlobalAlloc` impls in which unwinding is Undefined Behavior.

<!-- lint-subgroup-start: panic -->
Lints: `arithmetic_side_effects`, `expect_used`, `indexing_slicing`, `panic`, `string_slice`, `todo`, `unimplemented`, `unreachable`, `unwrap_used`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::string_slice,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
arithmetic_side_effects = "warn"
expect_used = "warn"
indexing_slicing = "warn"
panic = "warn"
string_slice = "warn"
todo = "warn"
unimplemented = "warn"
unreachable = "warn"
unwrap_used = "warn"
```
</details>
<!-- lint-subgroup-end -->

## `debug`

These are lints that can be useful to disable in CI, as they might indicate that code needs more work
or has remaining debugging artifacts.

<!-- lint-subgroup-start: debug -->
Lints: `dbg_macro`, `todo`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::dbg_macro,
    clippy::todo
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
dbg_macro = "warn"
todo = "warn"
```
</details>
<!-- lint-subgroup-end -->
