#![warn(clippy::raw_assign_to_drop)]

fn main() {
    unsafe fn foo(r: *mut String, i: *mut i32) {
        *r = "foo".to_owned();

        // no lint on {integer}
        *i = 47;

        (*r, *r) = ("foo".to_owned(), "bar".to_owned());

        (*r, *i) = ("foo".to_owned(), 47);

        let mut x: String = Default::default();
        *(&mut x as *mut _) = "Foo".to_owned();

        // no lint on `u8`
        *x.as_mut_ptr() = b'a';

        let mut v: Vec<String> = vec![];
        *v.as_mut_ptr() = Default::default();
    }
}
