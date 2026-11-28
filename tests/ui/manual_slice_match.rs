//@no-rustfix: suggestion is `MaybeIncorrect`
#![warn(clippy::manual_slice_match)]
#![allow(
    unused,
    clippy::comparison_to_empty,
    clippy::len_zero,
    clippy::never_loop,
    clippy::redundant_pattern_matching
)]

fn main() {}

fn vec_is_empty_then_len(v: Vec<u32>) {
    //~v manual_slice_match
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("{}", v[0]);
    } else {
        println!("many");
    }
}

fn slice_len_chain(s: &[u32]) {
    //~v manual_slice_match
    if s.len() == 0 {
        println!("empty");
    } else if s.len() > 1 {
        println!("{} {}", s[0], s[1]);
    } else {
        println!("one");
    }
}

// Should NOT lint: the receiver `s()` has side effects, so collapsing the per-branch
// calls into a single `match s() { .. }` would change how many times `s()` is evaluated.
fn slice_len_chain_impure(s: impl Fn() -> &'static [u32]) {
    if s().len() == 0 {
        println!("empty");
    } else if s().len() > 1 {
        println!("{} {}", s()[0], s()[1]);
    } else {
        println!("one");
    }
}

// Should NOT lint: `len() > 100` would need 101 placeholders, which is over
// `max-suggested-slice-pattern-length`, so the lint is suppressed.
fn over_pattern_length_limit(s: &[u32]) {
    if s.is_empty() {
        println!("empty");
    } else if s.len() > 100 {
        println!("large");
    } else {
        println!("small");
    }
}

// Should NOT lint: an array's length is a compile-time constant, so the chain is
// degenerate and a slice pattern of a different length would not type-check.
fn array_len_chain(a: [u32; 4]) {
    if a.len() == 1 {
        println!("one");
    } else if a.len() == 2 {
        println!("two");
    } else {
        println!("other");
    }
}

// Should NOT lint: `len() < n` cannot be expressed as a single slice pattern.
fn lt_fallback(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() < 3 {
        println!("few");
    } else {
        println!("many");
    }
}

// Should NOT lint: third condition is a value guard, not a length check
// (this is the uutils/coreutils#12822 shape).
fn value_guard(files: Vec<String>) {
    if files.is_empty() {
        println!("default");
    } else if files.len() > 1 {
        println!("extra: {}", files[1]);
    } else if files[0] == "-" {
        println!("stdin");
    } else {
        println!("{}", files[0]);
    }
}

// Should NOT lint: conditions inspect different receivers.
fn different_receivers(a: Vec<u32>, b: Vec<u32>) {
    if a.is_empty() {
        println!("a empty");
    } else if b.len() == 1 {
        println!("b one");
    } else {
        println!("other");
    }
}

// Should NOT lint: no final `else`.
fn no_final_else(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("one");
    }
}

// Should NOT lint: only a single condition.
fn single_condition(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else {
        println!("non-empty");
    }
}

// Should NOT lint: `len()` compared against a non-literal.
fn len_against_non_literal(v: Vec<u32>, n: usize) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == n {
        println!("n");
    } else {
        println!("other");
    }
}

// Should NOT lint: slice patterns were stabilised in 1.42.
#[clippy::msrv = "1.41"]
fn below_msrv(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("one");
    } else {
        println!("many");
    }
}

// Should lint: fields of locals are side-effect free receivers.
struct Wrapper {
    items: Vec<u32>,
}

fn field_receiver(w: &Wrapper) {
    //~v manual_slice_match
    if w.items.is_empty() {
        println!("empty");
    } else if w.items.len() == 1 {
        println!("one");
    } else {
        println!("many");
    }
}

// Should NOT lint: overloaded indexing may have side effects, and the `match` would
// evaluate `v[0]` only once instead of once per condition.
struct ChangingIndex(std::cell::Cell<usize>);
impl std::ops::Index<usize> for ChangingIndex {
    type Output = [u8];
    fn index(&self, _: usize) -> &[u8] {
        let n = self.0.get();
        self.0.set(n + 1);
        if n == 0 { &[1, 2] } else { &[1] }
    }
}

fn indexed(v: &ChangingIndex) -> &'static str {
    if v[0].is_empty() {
        "empty"
    } else if v[0].len() == 1 {
        "one"
    } else {
        "many"
    }
}

// Should NOT lint: `len` resolves to a user trait method, not to `Vec::len`.
trait PretendLen {
    fn len(self) -> usize;
}
impl PretendLen for &&Vec<u8> {
    fn len(self) -> usize {
        1
    }
}

fn shadow(v: &&Vec<u8>) -> &'static str {
    if v.is_empty() {
        "empty"
    } else if v.len() == 1 {
        "one"
    } else {
        "many"
    }
}

// Should lint, and keep the multi-line string literal untouched in the suggestion.
fn literal(v: &[u8]) -> &'static str {
    //~v manual_slice_match
    if v.is_empty() {
        "first
second"
    } else if v.len() == 1 {
        "one"
    } else {
        "many"
    }
}

// Should NOT lint: `[]` and `[_, ..]` already cover every length, so the final
// wildcard arm would be unreachable.
fn covered(v: &[u8]) -> &'static str {
    if v.is_empty() {
        "empty"
    } else if v.len() >= 1 {
        "nonempty"
    } else {
        "unreachable"
    }
}

// Should NOT lint: same as above with `[]`, `[_]` and `[_, _, ..]`.
fn covered_more(v: &[u8]) -> &'static str {
    if v.is_empty() {
        "empty"
    } else if v.len() == 1 {
        "one"
    } else if v.len() > 1 {
        "many"
    } else {
        "unreachable"
    }
}
