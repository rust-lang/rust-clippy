#![warn(clippy::paths_from_format)]

use std::path::PathBuf;

fn main() {
    let mut base_path1 = "";
    let mut base_path2 = "";
    PathBuf::from(format!("{base_path1}/foo/bar"));
    PathBuf::from(format!("/foo/bar/{base_path1}"));
    PathBuf::from(format!("/foo/{base_path1}/bar"));
    PathBuf::from(format!("foo/{base_path1}/bar"));
    PathBuf::from(format!("foo/foooo/{base_path1}/bar/barrr"));
    PathBuf::from(format!("foo/foooo/{base_path1}/bar/barrr/{base_path2}"));
    PathBuf::from(format!("{base_path2}/foo/{base_path1}/bar"));
    PathBuf::from(format!("foo/{base_path1}a/bar"));
    PathBuf::from(format!("foo/a{base_path1}/bar"));
    PathBuf::from(format!(r"C:\{base_path2}\foo\{base_path1}\bar"));
    PathBuf::from(format!("C:\\{base_path2}\\foo\\{base_path1}\\bar"));
}
