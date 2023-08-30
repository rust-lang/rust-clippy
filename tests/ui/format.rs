#![warn(clippy::useless_format)]
#![allow(
    unused_tuple_struct_fields,
    clippy::print_literal,
    clippy::redundant_clone,
    clippy::to_string_in_format_args,
    clippy::needless_borrow,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes,
    clippy::useless_vec
)]

struct Foo(pub String);

macro_rules! foo {
    ($($t:tt)*) => (Foo(format!($($t)*)))
}

fn main() {
    format!("foo");
    //~^ ERROR: useless use of `format!`
    //~| NOTE: `-D clippy::useless-format` implied by `-D warnings`
    format!("{{}}");
    //~^ ERROR: useless use of `format!`
    format!("{{}} abc {{}}");
    //~^ ERROR: useless use of `format!`
    format!(
    //~^ ERROR: useless use of `format!`
        r##"foo {{}}
" bar"##
    );

    let _ = format!("");
    //~^ ERROR: useless use of `format!`

    format!("{}", "foo");
    //~^ ERROR: useless use of `format!`
    format!("{:?}", "foo"); // Don't warn about `Debug`.
    format!("{:8}", "foo");
    format!("{:width$}", "foo", width = 8);
    format!("foo {}", "bar");
    format!("{} bar", "foo");

    let arg = String::new();
    format!("{}", arg);
    //~^ ERROR: useless use of `format!`
    format!("{:?}", arg); // Don't warn about debug.
    format!("{:8}", arg);
    format!("{:width$}", arg, width = 8);
    format!("foo {}", arg);
    format!("{} bar", arg);

    // We don’t want to warn for non-string args; see issue #697.
    format!("{}", 42);
    format!("{:?}", 42);
    format!("{:+}", 42);
    format!("foo {}", 42);
    format!("{} bar", 42);

    // We only want to warn about `format!` itself.
    println!("foo");
    println!("{}", "foo");
    println!("foo {}", "foo");
    println!("{}", 42);
    println!("foo {}", 42);

    // A `format!` inside a macro should not trigger a warning.
    foo!("should not warn");

    // Precision on string means slicing without panicking on size.
    format!("{:.1}", "foo"); // Could be `"foo"[..1]`
    format!("{:.10}", "foo"); // Could not be `"foo"[..10]`
    format!("{:.prec$}", "foo", prec = 1);
    format!("{:.prec$}", "foo", prec = 10);

    format!("{}", 42.to_string());
    //~^ ERROR: useless use of `format!`
    let x = std::path::PathBuf::from("/bar/foo/qux");
    format!("{}", x.display().to_string());
    //~^ ERROR: useless use of `format!`

    // False positive
    let a = "foo".to_string();
    let _ = Some(format!("{}", a + "bar"));
    //~^ ERROR: useless use of `format!`

    // Wrap it with braces
    let v: Vec<String> = vec!["foo".to_string(), "bar".to_string()];
    let _s: String = format!("{}", &*v.join("\n"));
    //~^ ERROR: useless use of `format!`

    format!("prepend {:+}", "s");

    // Issue #8290
    let x = "foo";
    let _ = format!("{x}");
    //~^ ERROR: useless use of `format!`
    let _ = format!("{x:?}"); // Don't lint on debug
    let _ = format!("{y}", y = x);
    //~^ ERROR: useless use of `format!`

    // Issue #9234
    let abc = "abc";
    let _ = format!("{abc}");
    //~^ ERROR: useless use of `format!`
    let xx = "xx";
    let _ = format!("{xx}");
    //~^ ERROR: useless use of `format!`
}
