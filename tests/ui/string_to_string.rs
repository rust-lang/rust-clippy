#![warn(clippy::string_to_string)]
#![allow(clippy::redundant_clone, clippy::unnecessary_literal_unwrap)]

fn main() {
    let mut message = String::from("Hello");
    let mut v = message.to_string();
    //~^ string_to_string

    let variable1 = String::new();
    let v = &variable1;
    let variable2 = Some(v);
    let _ = variable2.map(|x| {
        println!();
        x.to_string()
    });
    //~^^ string_to_string

    let x = Some(String::new());
    let _ = x.unwrap_or_else(|| v.to_string());
    //~^ string_to_string
}

mod issue15300 {
    use std::collections::{BTreeMap, HashSet};

    struct SourceFile {
        url: String,
        ty: u32,
    }

    fn wrong_cloned(sources: BTreeMap<String, SourceFile>) {
        let _ = sources
            .iter()
            .map(|x| x.1)
            .filter(|x| x.ty == 0)
            .map(|x| x.url.to_string())
            //~^ string_to_string
            .collect::<HashSet<_>>();
    }
}
