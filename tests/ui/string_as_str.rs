#![allow(unused)]
#![warn(clippy::string_as_str)]

use clippy_utils::consts::Constant;

fn main() {
    let my_string = String::from("hey");
    let s = my_string.as_str();
    fn_with_str(my_string.as_str());

    let fs = FakeString::default();
    let ss = fs.as_str();

    match my_string.as_str() {
        "hello" => (),
        "hey" => (),
        _ => (),
    }

    match (my_string.as_str(), my_string.as_str()) {
        ("hello", ("hello")) => (),
        ("hey", "hey") => (),
        _ => (),
    }

    const ARRAY: &[&str] = &["hello", "dump_hir"];
    let s: String = "hello".to_string();
    let exists = ARRAY.contains(&s.as_str());

    let a = s.as_str().chars().map(|c| c.is_digit(2)).collect::<Vec<_>>();

    let path = vec!["path1".to_string(), "path2".to_string()];
    let path: Vec<&str> = path.iter().map(|x| x.as_str()).collect();
    let snip = "String".to_owned();
    let name = "alice".to_string();
    let a: Option<&str> = match snip.split_once(" as ") {
        None => Some(snip.as_str()),
        Some((import, rename)) => {
            if rename.trim() == name {
                None
            } else {
                Some(import.trim())
            }
        },
    };
}

fn fn_with_str(my_string: &str) {
    println!("hey {my_string}");
}

#[derive(Default)]
struct FakeString(pub u32);

impl FakeString {
    fn as_str(&self) -> String {
        self.0.to_string()
    }
}
