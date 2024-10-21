# More granular lint groups

Clippy groups its lints into [9 primary categories](lints.md),
two of which are allow-by-default (pedantic and restriction).

One downside with having such few but broad categories for allow-by-default lints
is that it significantly decreases discoverability, as `restriction` often acts as a blanket category
for any lint that most users likely would not want enforced on their codebase.

This page should help with that, by defining more granular, unofficial lint groups.
For example, some people might not be interested in all the style-related `pedantic` lints,
but *are* interested in the `perf`-related ones, so it can be worth going through some of these.

<!--
NOTE: Do not edit the contents in between lint-group-start and lint-group-end manually.
These sections are generated based on the `GROUPS` array defined in tests/lint-groups.rs,
so consider updating that instead and re-running `cargo bless`.
The descriptions however are fine to edit.
-->

## Perf-pedantic

These are `pedantic` lints that look for code patterns that could be expressed in a more efficient way.
These would be candidates for the `perf` category, however suggestions made by them can also sometimes hurt readability
and obfuscate the meaning, so occasional `#[allow]`s are expected to be used.

<!-- lint-group-start: perf-pedantic -->
Lints: `assigning_clones`, `inefficient_to_string`, `naive_bytecount`, `needless_bitwise_bool`, `trivially_copy_pass_by_ref`, `unnecessary_join`, `unnecessary_box_returns`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::assigning_clones,
    clippy::inefficient_to_string,
    clippy::naive_bytecount,
    clippy::needless_bitwise_bool,
    clippy::trivially_copy_pass_by_ref,
    clippy::unnecessary_join,
    clippy::unnecessary_box_returns
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
unnecessary_join = "warn"
unnecessary_box_returns = "warn"
```
</details>
<!-- lint-group-end: perf-pedantic -->


## Perf-restriction

These are `restriction` lints that can improve the performance of code, but are very specific
and sometimes *significantly* hurt readability with very little gain in the usual case.
These should ideally only be applied to specific functions or modules that were profiled
and where it is very clear that any performance gain matters.

As always (but especially here), you should double-check that applying these actually helps
and that any performance wins are worth the introduced complexity.

<!-- lint-group-start: perf-restriction -->
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
<!-- lint-group-end: perf-restriction -->

## Perf-nursery

These are `nursery` lints that either were previously in the `perf` category or are intended to be in `perf`
but have too many false positives.
Some of them may also be simply wrong in certain situations and end up slower,
so you should make sure to read the description to learn about possible edge cases.

<!-- lint-group-start: perf-nursery -->
Lints: `redundant_clone`, `iter_with_drain`, `mutex_integer`, `or_fun_call`, `significant_drop_tightening`, `trivial_regex`, `needless_collect`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::redundant_clone,
    clippy::iter_with_drain,
    clippy::mutex_integer,
    clippy::or_fun_call,
    clippy::significant_drop_tightening,
    clippy::trivial_regex,
    clippy::needless_collect
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
redundant_clone = "warn"
iter_with_drain = "warn"
mutex_integer = "warn"
or_fun_call = "warn"
significant_drop_tightening = "warn"
trivial_regex = "warn"
needless_collect = "warn"
```
</details>
<!-- lint-group-end: perf-nursery -->

## Panicking

These are `restriction` lints that look for patterns that can introduce panics.

Usually panics are not something that one should want to avoid and most of the time panicking is perfectly valid
(hence why these lints are allow-by-default),
but users may want to forbid any use of panicky functions altogether in specific contexts.

One use case could be to annotate `GlobalAlloc` impls in which unwinding is Undefined Behavior.

<!-- lint-group-start: panicking -->
Lints: `arithmetic_side_effects`, `expect_used`, `unwrap_used`, `panic`, `unreachable`, `todo`, `unimplemented`, `string_slice`, `indexing_slicing`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::string_slice,
    clippy::indexing_slicing
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
arithmetic_side_effects = "warn"
expect_used = "warn"
unwrap_used = "warn"
panic = "warn"
unreachable = "warn"
todo = "warn"
unimplemented = "warn"
string_slice = "warn"
indexing_slicing = "warn"
```
</details>
<!-- lint-group-end: panicking -->

## Debugging

These are lints that can be useful to disable in CI, as they might indicate that code needs more work
or has remaining debugging artifacts.

<!-- lint-group-start: debugging -->
Lints: `dbg_macro`, `todo`, `unimplemented`

<details>
<summary>#![warn] attribute</summary>

```
#![warn(
    clippy::dbg_macro,
    clippy::todo,
    clippy::unimplemented
)]
```
</details>

<details>
<summary>Lint table</summary>

```
[lints.clippy]
dbg_macro = "warn"
todo = "warn"
unimplemented = "warn"
```
</details>
<!-- lint-group-end: debugging -->
