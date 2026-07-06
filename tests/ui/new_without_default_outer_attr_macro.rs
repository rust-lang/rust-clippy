//@aux-build:proc_macro_attr.rs

// The suggestion here must not be machine-applicable: applying it verbatim would move
// `#[proc_macro_attr::fake_pymethods]` onto the newly suggested `impl Default` block instead of
// leaving it on the original inherent impl, which fails to compile since the attribute (like
// PyO3's `#[pymethods]`) cannot be applied to a trait impl. See rust-lang/rust-clippy#17361.
//@no-rustfix

#![warn(clippy::new_without_default)]

extern crate proc_macro_attr;

pub struct Foo;

#[proc_macro_attr::fake_pymethods]
impl Foo {
    pub fn new() -> Self {
        //~^ new_without_default
        Foo
    }
}

// Regression test: the attribute's arguments contain a `]` inside a string literal. A naive
// bracket-counting scan of the source line would close on that `]` instead of the one that
// actually ends the attribute, mis-parse the rest of the line, and conclude the line isn't a
// standalone attribute after all -- silently reintroducing the exact bug above for any consumed
// attribute macro whose args contain a `]` in a string (e.g. `#[cfg(feature = "x]y")]`).
pub struct Baz;

#[proc_macro_attr::dummy(note = "]")]
impl Baz {
    pub fn new() -> Self {
        //~^ new_without_default
        Baz
    }
}

// Regression test: a trailing `// comment` after the attribute contains a `]`. Scanning the whole
// line (including the comment) for the last `]` would land on the one in the comment text instead
// of the attribute's own closing bracket, leave real comment text in the "tail", and wrongly
// conclude the line isn't standalone after all -- silently reintroducing the exact bug above for
// any consumed attribute macro with a bracket-containing trailing comment.
pub struct Qux;

#[proc_macro_attr::dummy(unused)] // uses obj[idx] pattern
impl Qux {
    pub fn new() -> Self {
        //~^ new_without_default
        Qux
    }
}

// Regression test: the attribute's arguments contain a `//` inside a string literal. That must
// not be mistaken for the start of a trailing line comment (which would then wrongly get
// stripped along with everything after it, including the attribute's own closing bracket).
pub struct Quux;

#[proc_macro_attr::dummy(note = "http://example.com")]
impl Quux {
    pub fn new() -> Self {
        //~^ new_without_default
        Quux
    }
}

// Regression test: the attribute's arguments contain a raw string whose content has a `"`
// followed by ` // `. A scan that only tracks plain (non-raw) string state closes the "string"
// on that inner `"`, then mistakes the real ` // bar"#)]` tail for a line comment and strips it,
// which breaks the bracket scan and wrongly concludes the line isn't standalone after all --
// silently reintroducing the exact bug above.
pub struct Corge;

#[proc_macro_attr::dummy(note = r#"foo" // bar"#)]
impl Corge {
    pub fn new() -> Self {
        //~^ new_without_default
        Corge
    }
}

// Regression test: the attribute's own closing `)]` lands on a line of its own, above the impl.
// Classifying only that single closing line (which starts with `)`, not `#`) would wrongly
// conclude there's no attribute at all above the impl, reintroducing the exact bug above for any
// multi-line consumed attribute macro invocation.
pub struct Grault;

#[proc_macro_attr::fake_pymethods(
    note
)]
impl Grault {
    pub fn new() -> Self {
        //~^ new_without_default
        Grault
    }
}

// Regression test: a block comment sits between the attribute and the impl. A caller that only
// skips `//` line comments (not `/* .. */` block comments) while walking upward would stop right
// at this comment line and never see the attribute above it, reintroducing the exact bug above.
pub struct Garply;

#[proc_macro_attr::fake_pymethods]
/* comment */
impl Garply {
    pub fn new() -> Self {
        //~^ new_without_default
        Garply
    }
}

// Same as above, but the block comment itself spans multiple lines.
pub struct Waldo;

#[proc_macro_attr::fake_pymethods]
/* multi
   line
   comment */
impl Waldo {
    pub fn new() -> Self {
        //~^ new_without_default
        Waldo
    }
}

// Regression test: real code sits *directly* above the attribute, with no blank line at all
// separating them. This is the actual PyO3 shape this lint exists to protect
// (`#[pyclass] struct Foo;` immediately followed by `#[pymethods] impl Foo`), and every test
// above this one happens to have a blank line between the preceding item and the attribute --
// so none of them would have caught a check that only looked for a blank line as the boundary
// of "the attribute above the impl". See rust-lang/rust-clippy#17361.
pub struct Fred;
#[proc_macro_attr::fake_pymethods]
impl Fred {
    pub fn new() -> Self {
        //~^ new_without_default
        Fred
    }
}

// Same regression, but the real code directly above the attribute (no blank line) is the closing
// brace of a preceding function rather than a struct declaration.
pub struct Plugh;

pub fn helper_for_plugh() {}
#[proc_macro_attr::fake_pymethods]
impl Plugh {
    pub fn new() -> Self {
        //~^ new_without_default
        Plugh
    }
}

// Regression test: a `//` comment sits directly above the impl (no blank line between the
// comment and the impl), with the attribute above that separated by a blank line. Same
// "no-blank-line-at-the-final-boundary" shape as the two tests above, just with a comment as the
// last thing before the impl instead of nothing.
pub struct Xyzzy;

#[proc_macro_attr::fake_pymethods]

// note about the impl
impl Xyzzy {
    pub fn new() -> Self {
        //~^ new_without_default
        Xyzzy
    }
}

fn main() {}
