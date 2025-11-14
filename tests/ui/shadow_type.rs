#![warn(clippy::shadow_type_generic)]

pub mod structs {
    struct Foo;
    enum Bar {}

    //~v shadow_type_generic
    struct Struct1<Foo> {
        foo: Foo,
    }
    //~v shadow_type_generic
    struct Struct2<Bar> {
        bar: Bar,
    }
    //~v shadow_type_generic
    struct Struct3<Foo, Bar> {
        foo: Foo,
        bar: Bar,
    }
    //~v shadow_type_generic
    struct Struct4<Foo, B, Bar> {
        foo: Foo,
        b: B,
        bar: Bar,
    }
    struct Struct5 {
        foo: Foo,
    }
    struct Struct6 {
        bar: Bar,
    }
}

pub mod enums {
    struct Foo;
    enum Bar {}

    //~v shadow_type_generic
    enum Enum1<Foo> {
        Foo(Foo),
    }
    //~v shadow_type_generic
    enum Enum2<Bar> {
        Bar(Bar),
    }
    //~v shadow_type_generic
    enum Enum3<Foo, Bar> {
        Foo(Foo),
        Bar(Bar),
    }
    //~v shadow_type_generic
    enum Enum4<Foo, B, Bar> {
        Foo(Foo),
        B(B),
        Bar(Bar),
    }
    enum Enum5 {
        Foo(Foo),
    }
    enum Enum6 {
        Bar(Bar),
    }
}

fn main() {}
