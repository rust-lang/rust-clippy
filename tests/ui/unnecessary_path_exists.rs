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
        let _ = path.is_symlink();
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

fn main() {}
