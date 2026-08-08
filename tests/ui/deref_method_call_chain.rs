#![warn(clippy::deref_method_call_chain)]
#![allow(clippy::needless_borrow, clippy::redundant_as_str, clippy::into_iter_on_ref)]

mod issue2094 {
    use std::ffi::{CString, OsString};
    use std::ops::Deref;
    use std::path::{Path, PathBuf};

    struct DerefWrapper(Vec<u8>);

    impl Deref for DerefWrapper {
        type Target = [u8];
        fn deref(&self) -> &[u8] {
            &self.0
        }
    }

    impl DerefWrapper {
        fn as_slice(&self) -> &[u8] {
            &self.0
        }
    }

    struct Shadowing(Vec<u8>);

    impl Deref for Shadowing {
        type Target = [u8];
        fn deref(&self) -> &[u8] {
            &self.0
        }
    }

    impl Shadowing {
        fn as_slice(&self) -> &[u8] {
            &self.0
        }

        fn first(&self) -> u32 {
            42
        }
    }

    struct NotDeref(String);

    impl NotDeref {
        fn as_str(&self) -> &str {
            &self.0
        }
    }

    struct DerefOnly(Vec<u8>);

    impl Deref for DerefOnly {
        type Target = [u8];
        fn deref(&self) -> &[u8] {
            &self.0
        }
    }

    impl DerefOnly {
        fn as_mut_slice(&mut self) -> &mut [u8] {
            &mut self.0
        }
    }

    macro_rules! in_macro {
        ($v:expr) => {
            $v.as_slice().first()
        };
    }

    fn check() {
        let mut vec = vec![1, 2, 3];
        let mut string = String::from("hello world");
        let pathbuf = PathBuf::from("path");
        let os_string = OsString::from("hello");
        let c_string = CString::new("hello").unwrap();

        // The conversion is redundant: the method exists on the deref target and
        // is found on the original receiver through deref coercion
        let _ = vec.as_slice().first();
        //~^ deref_method_call_chain
        let _ = vec.as_slice().len();
        //~^ deref_method_call_chain
        vec.as_mut_slice().sort_unstable();
        //~^ deref_method_call_chain
        let _ = string.as_str().find('l');
        //~^ deref_method_call_chain
        let _ = string.as_str().parse::<i32>();
        //~^ deref_method_call_chain
        string.as_mut_str().make_ascii_uppercase();
        //~^ deref_method_call_chain
        let _ = pathbuf.as_path().exists();
        //~^ deref_method_call_chain
        let _ = os_string.as_os_str().len();
        //~^ deref_method_call_chain
        let _ = c_string.as_c_str().to_bytes();
        //~^ deref_method_call_chain
        let _ = (&vec).as_slice().first();
        //~^ deref_method_call_chain
        let vec_ref: &Vec<i32> = &vec;
        let _ = vec_ref.as_slice().first();
        //~^ deref_method_call_chain
        let _ = vec.as_slice().first().copied();
        //~^ deref_method_call_chain

        // User types with a deref-equivalent conversion are detected too
        let wrapper = DerefWrapper(vec![1, 2, 3]);
        let _ = wrapper.as_slice().first();
        //~^ deref_method_call_chain

        // Trait methods could resolve to a different impl on the original receiver
        let _ = vec.as_slice().into_iter();
        let _ = string.as_str().to_owned();
        let _ = string.as_str().to_string();

        // Covered by `redundant_as_str`
        let _ = string.as_str().is_empty();
        let _ = string.as_str().as_bytes();

        // Arrays don't implement `Deref`
        let _ = [1, 2, 3].as_slice().first();

        // `Path` doesn't implement `Deref`
        let path: &Path = pathbuf.as_path();
        let _ = path.as_os_str().len();

        // Not a deref-equivalent conversion (receiver doesn't implement `Deref`)
        let not_deref = NotDeref(String::from("hi"));
        let _ = not_deref.as_str().len();

        // `as_mut_slice` needs `DerefMut` to be removable, but this type only implements `Deref`
        let mut deref_only = DerefOnly(vec![1, 2, 3]);
        deref_only.as_mut_slice().sort_unstable();

        // The receiver type has its own `first`, which would shadow `<[u8]>::first`
        let shadowing = Shadowing(vec![1, 2, 3]);
        let _ = shadowing.as_slice().first();

        // No method follows the conversion
        let _ = vec.as_slice();

        // Not linted inside macro expansions
        let _ = in_macro!(vec);
    }
}

fn main() {}
