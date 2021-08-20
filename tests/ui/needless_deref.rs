// run-rustfix

#![allow(dead_code)]
#![warn(clippy::needless_deref)]

fn main() {}

mod immutable {
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn foo(_: &Path) {}
    fn bar<T>(_: T) {}

    // Arc<PathBuf> -> PathBuf -> Path
    fn test_call() {
        {
            let a = Arc::new(PathBuf::new());
            foo(&**a); // should not lint

            let a = &Arc::new(PathBuf::new());
            foo(&***a); // should not lint

            foo(&*PathBuf::new()); // should lint

            let b = &PathBuf::new();
            foo(b); // should not lint
            foo(&*b); // should not lint
            foo(&**b); // should lint
        }
        {
            let a = Arc::new(PathBuf::new());
            bar(&**a); // should not lint

            let a = &Arc::new(PathBuf::new());
            bar(&***a); // should not lint

            bar(&*PathBuf::new()); // should not lint

            let b = &PathBuf::new();
            bar(b); // should not lint
            bar(&*b); // should not lint
            bar(&**b); // should not lint
        }
        {
            let b = &PathBuf::new();
            let z = &**b;
            foo(z); // should lint, false negative
        }
    }

    struct S;
    impl S {
        fn foo(&self, _a: &Path) {}
        fn bar<T>(&self, _a: T) {}
    }

    fn test_method_call() {
        let s = S;
        {
            let a = Arc::new(PathBuf::new());
            s.foo(&**a); // should not lint

            let a = &Arc::new(PathBuf::new());
            s.foo(&***a); // should not lint

            s.foo(&*PathBuf::new()); // should lint

            let b = &PathBuf::new();
            s.foo(b); // should not lint
            s.foo(&*b); // should not lint
            s.foo(&**b); // should lint
        }
        {
            let a = Arc::new(PathBuf::new());
            s.bar(&**a); // should not lint

            let a = &Arc::new(PathBuf::new());
            s.bar(&***a); // should not lint

            s.bar(&*PathBuf::new()); // should not lint

            let b = &PathBuf::new();
            s.bar(b); // should not lint
            s.bar(&*b); // should not lint
            s.bar(&**b); // should not lint
        }
    }
}

mod mutable {
    fn foo(_: &mut usize) {}
    fn bar<T>(_: T) {}

    fn test_call() {
        {
            let mut b = Box::new(0);
            foo(&mut b); // should not lint
            foo(&mut *b); // should lint
            let b = &mut Box::new(0);
            foo(b); // should not lint
            foo(&mut *b); // should not lint
            foo(&mut **b); // should lint
        }
        {
            let mut b = Box::new(0);
            bar(&mut b); // should not lint
            bar(&mut *b); // should not lint
            let b = &mut Box::new(0);
            bar(b); // should not lint

            let b = &mut Box::new(0);
            bar(&mut *b); // should not lint

            let b = &mut Box::new(0);
            bar(&mut **b); // should not lint
        }
    }

    struct S;
    impl S {
        fn foo(&self, _a: &mut usize) {}
        fn bar<T>(&self, _a: T) {}
    }

    fn test_method_call() {
        let s = S;
        {
            let mut b = Box::new(0);
            s.foo(&mut b); // should not lint
            s.foo(&mut *b); // should lint
            let b = &mut Box::new(0);
            s.foo(b); // should not lint
            s.foo(&mut *b); // should not lint
            s.foo(&mut **b); // should lint
        }
        {
            let mut b = Box::new(0);
            s.bar(&mut b); // should not lint
            s.bar(&mut *b); // should not lint
            let b = &mut Box::new(0);
            s.bar(b); // should not lint

            let b = &mut Box::new(0);
            s.bar(&mut *b); // should not lint

            let b = &mut Box::new(0);
            s.bar(&mut **b); // should not lint
        }
    }
}

mod code_bloating {
    use std::fmt::Display;

    fn main() {
        let a = &String::new();
        foo(&**a); // should lint
        bar(&**a); // should not lint. Otherwise `bar` has to be generialized twice for `bar(&**a)`(T:&String) and `bar(b)`(T:&str)
        // This is a code bloating problem.
        let b: &str = "";
        bar(b);
    }

    fn foo(_: &str) {}

    fn bar<T: Display>(_: T) {}
}
