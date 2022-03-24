// run-rustfix

#![warn(clippy::unnecessary_join)]

fn main() {
    // should be linted
    let vector = vec!["hello", "world"];
    let output = vector
        .iter()
        .map(|item| item.to_uppercase())
        .collect::<Vec<String>>()
        .join("");
    println!("{}", output);

    // should be linted
    let vector = vec!["hello", "world"];
    let output = vector
        .iter()
        .map(|item| item.to_uppercase())
        .collect::<Vec<_>>()
        .join("");
    println!("{}", output);

    // should not be linted
    let vector = vec!["hello", "world"];
    let output = vector
        .iter()
        .map(|item| item.to_uppercase())
        .collect::<Vec<String>>()
        .join("\n");
    println!("{}", output);

    // should not be linted
    let vector = vec!["hello", "world"];
    let output = vector.iter().map(|item| item.to_uppercase()).collect::<String>();
    println!("{}", output);
}
