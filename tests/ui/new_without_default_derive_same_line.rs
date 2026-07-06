// A `#[derive(..)]` (or any other outer attribute) that shares its source line with the item
// it annotates -- e.g. `#[derive(Debug)] pub struct Foo;` -- belongs to that item, not to the
// `impl` block written on the next line. The lint's source-sniffing fallback (which exists to
// catch outer attribute *macros* fully consumed by expansion, see
// new_without_default_outer_attr_macro.rs) must not mistake this for such a case and must keep
// suggesting a machine-applicable fix.

#![warn(clippy::new_without_default)]

#[derive(Debug)]
pub struct Foo;

impl Foo {
    pub fn new() -> Self {
        //~^ new_without_default
        Foo
    }
}

// Same false positive, but with the derive and the struct crammed onto one line, which is the
// exact case that used to be misdetected as a hidden attribute on the `impl` below.
#[rustfmt::skip]
#[derive(Debug)] pub struct Bar;

impl Bar {
    pub fn new() -> Self {
        //~^ new_without_default
        Bar
    }
}

// Same false positive again, but the attribute's argument contains a `]` inside a string
// literal. A naive bracket-counting scan would close on that `]` and misparse the rest of the
// line as trailing code, wrongly concluding this attribute belongs to the `impl` below instead
// of `Baz`.
#[rustfmt::skip]
#[doc = "]"] pub struct Baz;

impl Baz {
    pub fn new() -> Self {
        //~^ new_without_default
        Baz
    }
}

// Same false positive again, but this time the line also carries a trailing `// comment` that
// itself contains a `]`. Scanning the whole line (including the comment) for the last `]` would
// land on the one in the comment instead of the attribute's own closing bracket, and then
// misparse the real `pub struct Qux;` code as if it were more comment text, wrongly concluding
// the attribute is standalone and belongs to the `impl` below instead of `Qux`.
#[rustfmt::skip]
#[doc = "]"] pub struct Qux; // trailing comment with a bracket ]

impl Qux {
    pub fn new() -> Self {
        //~^ new_without_default
        Qux
    }
}

fn main() {}
