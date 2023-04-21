//@run-rustfix

#![warn(clippy::unwrap_or_else_default)]
#![allow(dead_code)]
#![allow(clippy::unnecessary_wraps)]

/// Checks implementation of the `UNWRAP_OR_ELSE_DEFAULT` lint.
fn unwrap_or_else_default() {
    struct Foo;

    impl Foo {
        fn new() -> Foo {
            Foo
        }

        // fake default, we should not trigger on this
        fn default() -> Foo {
            Foo
        }
    }

    struct HasDefaultAndDuplicate;

    impl HasDefaultAndDuplicate {
        fn default() -> Self {
            HasDefaultAndDuplicate
        }
    }

    impl Default for HasDefaultAndDuplicate {
        fn default() -> Self {
            HasDefaultAndDuplicate
        }
    }

    enum Enum {
        A(),
    }

    fn make<T, V>(_: V) -> T {
        unimplemented!();
    }

    let with_enum = Some(Enum::A());
    with_enum.unwrap_or_else(Enum::A);

    let with_new = Some(vec![1]);
    with_new.unwrap_or_else(Vec::new);

    let with_err: Result<_, ()> = Ok(vec![1]);
    with_err.unwrap_or_else(make);

    // should not be changed
    let with_fake_default = None::<Foo>;
    with_fake_default.unwrap_or_else(Foo::default);

    // should not be changed
    let with_fake_default2 = None::<HasDefaultAndDuplicate>;
    with_fake_default2.unwrap_or_else(<HasDefaultAndDuplicate>::default);

    let with_real_default = None::<HasDefaultAndDuplicate>;
    with_real_default.unwrap_or_else(<HasDefaultAndDuplicate as Default>::default);

    let with_default_trait = Some(1);
    with_default_trait.unwrap_or_else(Default::default);

    let with_default_type = Some(1);
    with_default_type.unwrap_or_else(u64::default);

    let with_default_type: Option<Vec<u64>> = None;
    with_default_type.unwrap_or_else(Vec::new);

    let empty_string = None::<String>;
    empty_string.unwrap_or_else(|| "".to_string());
}

fn main() {}
