#![warn(clippy::needless_deref)]

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    test_call();
    test_method_call();
}

// Arc<PathBuf> -> PathBuf -> Path
fn test_call() {
    fn foo(_: &Path) {}
    fn bar<T>(_: T) {}

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
