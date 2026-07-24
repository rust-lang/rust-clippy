#![warn(clippy::unnecessary_path_exists)]

use std::path::{Path, PathBuf};

fn check_path(path: &Path) {
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.is_file();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.is_dir();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.canonicalize().unwrap();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.read_dir().unwrap();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.symlink_metadata().unwrap();
    }

    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.read_link().unwrap();
    }

    // has an else branch — TOCTOU race still present
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    } else {
        println!("path does not exist");
    }

    // no filesystem operation in body — no lint
    if path.exists() {
        println!("path exists");
    }
}

fn check_pathbuf(path: PathBuf) {
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    }
}

fn check_different_receiver(path: &Path, other: &Path) {
    // different receiver — no lint
    if path.exists() {
        let _ = other.metadata().unwrap();
    }
}

fn check_with_result(path: &Path) -> std::io::Result<()> {
    // ? operator
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata()?;
    }
    Ok(())
}

fn check_try_exists(path: &Path) -> std::io::Result<()> {
    // `try_exists()?` as a direct condition
    if path.try_exists()? {
        //~^ unnecessary_path_exists
        let _ = path.metadata()?;
    }

    // `try_exists()?` in a compound condition
    if path.try_exists()? && true {
        //~^ unnecessary_path_exists
        let _ = path.metadata()?;
    }

    // `try_exists()?` stored in a bool, immediately followed by if
    let exists = path.try_exists()?;
    //~^ unnecessary_path_exists
    if exists {
        let _ = path.metadata()?;
    }

    // `.unwrap_or(false)` instead of `?` — not detected (known limitation)
    if path.try_exists().unwrap_or(false) {
        let _ = path.metadata()?;
    }

    Ok(())
}

fn check_statement_forms(path: &Path) {
    // no let binding
    if path.exists() {
        //~^ unnecessary_path_exists
        path.metadata().ok();
    }

    // fs op not the first statement
    if path.exists() {
        //~^ unnecessary_path_exists
        println!("checking path");
        let _ = path.metadata().unwrap();
    }

    // deeper method chain
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().ok().is_some();
    }
}

fn check_compound_condition(path: &Path) {
    let condition = true;

    // exists() on the left side of &&
    if path.exists() && condition {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    }

    // exists() on the right side of &&
    if condition && path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    }
}

fn check_stored_bool(path: &Path) {
    // stored bool, immediately followed by if
    let exists = path.exists();
    //~^ unnecessary_path_exists
    if exists {
        let _ = path.metadata().unwrap();
    }

    // stored bool with PathBuf
    let path2 = PathBuf::from("test");
    let exists2 = path2.exists();
    //~^ unnecessary_path_exists
    if exists2 {
        let _ = path2.metadata().unwrap();
    }

    // stored bool with else branch
    let exists3 = path.exists();
    //~^ unnecessary_path_exists
    if exists3 {
        let _ = path.metadata().unwrap();
    } else {
        println!("not found");
    }

    // stored bool with compound condition — no lint (condition is not the plain local)
    let exists4 = path.exists();
    if exists4 && true {
        let _ = path.metadata().unwrap();
    }
}

fn check_stored_bool_not_immediate(path: &Path) {
    // intervening statement — do not lint
    let exists = path.exists();
    println!("something in between");
    if exists {
        let _ = path.metadata().unwrap();
    }
}

fn check_false_positives(path: &Path) {
    // non-fs method — no lint
    if path.exists() {
        let _ = path.display().to_string();
    }

    // free function call, not a method on the receiver — no lint
    if path.exists() {
        let _ = std::fs::read(path);
    }

    // negated condition — the `exists()` result being true isn't what guards the `then`
    // branch, so no lint
    if !path.exists() {
        let _ = path.metadata();
    }

    // `||` instead of `&&` — `exists()` being true doesn't guarantee the `then` branch only
    // runs when the path exists, so no lint
    if path.exists() || true {
        let _ = path.metadata();
    }

    // fs call only in the `else` branch — the `exists()` check guards the *other* branch,
    // so no lint
    if path.exists() {
        println!("path exists");
    } else {
        let _ = path.metadata();
    }

    // `path` is shadowed between the `exists()` check and the filesystem call, so the
    // filesystem call is on a different local — no lint
    if path.exists() {
        let path = Path::new("other");
        let _ = path.metadata();
    }

    // `is_symlink()` doesn't follow the symlink the way `exists()` does, so it isn't checking
    // the same thing — no lint
    if path.exists() {
        let _ = path.is_symlink();
    }
}

fn dyn_path() -> &'static Path {
    Path::new("dyn")
}

fn check_call_receiver() {
    // the receiver is a function call, not a stable place — nothing guarantees the two calls
    // return the same path, so no lint
    if dyn_path().exists() {
        let _ = dyn_path().metadata();
    }
}

fn check_reassigned_receiver(path: &Path) {
    let mut path_clone = PathBuf::from(path);
    // `path_clone` is reassigned before the filesystem call, so the `exists()` result no longer
    // describes the value `metadata()` ends up seeing — no lint
    if path_clone.exists() {
        path_clone = PathBuf::new();
        let _ = path_clone.metadata();
    }
}

fn check_mutated_receiver(path: &Path) {
    let mut path_clone = PathBuf::from(path);
    // `path_clone` is mutated in place (not reassigned) before the filesystem call — same
    // problem as `check_reassigned_receiver`, just via `PathBuf::push` instead of `=` — no lint
    if path_clone.exists() {
        path_clone.push("subdir");
        let _ = path_clone.metadata();
    }
}

fn check_mutated_receiver_after_fs_call(path: &Path) {
    let mut path_clone = PathBuf::from(path);
    // the mutation happens after the filesystem call, so it doesn't affect this lint — still
    // lints
    if path_clone.exists() {
        //~^ unnecessary_path_exists
        let _ = path_clone.metadata().unwrap();
        path_clone.push("subdir");
    }
}

fn check_iterator_receiver(mut iter: impl Iterator<Item = PathBuf>) -> Option<()> {
    // `iter.next()` mutates the iterator and can return a different path each call — no lint
    if iter.next()?.exists() {
        let _ = iter.next()?.metadata();
    }
    Some(())
}

fn check_closure_deferred(path: &Path) {
    // the filesystem call is inside a closure, which isn't necessarily ever called — no lint
    if path.exists() {
        let _ = || {
            let _ = path.metadata();
        };
    }
}

fn check_nested_then_block(path: &Path) {
    // the filesystem call is nested inside another `if`, but that doesn't defer execution the
    // way a closure does — still lints
    if path.exists() {
        //~^ unnecessary_path_exists
        if true {
            let _ = path.metadata().unwrap();
        }
    }
}

struct Custom;

impl Custom {
    fn exists(&self) -> bool {
        true
    }

    fn metadata(&self) -> u32 {
        0
    }
}

fn check_unrelated_exists_method(c: Custom) {
    // `exists`/`metadata` here are unrelated to `Path` — no lint
    if c.exists() {
        let _ = c.metadata();
    }
}

use std::ops::Deref;

struct PathWrapper(PathBuf);

impl Deref for PathWrapper {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

fn check_deref_to_path(path: PathWrapper) {
    // `exists`/`metadata` resolve through `Deref<Target = Path>` — still lints
    if path.exists() {
        //~^ unnecessary_path_exists
        let _ = path.metadata().unwrap();
    }
}

macro_rules! exists_then_metadata {
    ($path:expr) => {
        if $path.exists() {
            let _ = $path.metadata().unwrap();
        }
    };
}

fn check_macro_generates_whole_pattern(path: &Path) {
    // the entire `if`/`exists`/`metadata` pattern comes from a macro expansion — no lint
    exists_then_metadata!(path);
}

macro_rules! path_exists {
    ($path:expr) => {
        $path.exists()
    };
}

fn check_macro_generates_condition(path: &Path) {
    // the `exists()` call itself comes from a macro expansion — no lint (the `Methods` lint
    // pass skips any expression whose span originates from a macro)
    if path_exists!(path) {
        let _ = path.metadata().unwrap();
    }
}

macro_rules! read_metadata {
    ($path:expr) => {
        let _ = $path.metadata().unwrap();
    };
}

fn check_macro_generates_fs_call(path: &Path) {
    // the filesystem call comes from a macro, but the `if`/`exists()` are written directly
    if path.exists() {
        //~^ unnecessary_path_exists
        read_metadata!(path);
    }
}

fn main() {}
