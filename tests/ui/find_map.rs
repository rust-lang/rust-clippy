#![warn(clippy::all, clippy::pedantic)]

#[derive(Debug, Copy, Clone)]
enum Flavor {
    Chocolate,
}

#[derive(Debug, Copy, Clone)]
enum Dessert {
    Banana,
    Pudding,
    Cake(Flavor),
}

fn main() {
    let desserts_of_the_week = vec![Dessert::Banana, Dessert::Cake(Flavor::Chocolate), Dessert::Pudding];

    let a = ["lol", "NaN", "2", "5", "Xunda"];

    let _: Option<i32> = a.iter().find(|s| s.parse::<i32>().is_ok()).map(|s| s.parse().unwrap());
}

fn no_lint() {
    let _ = vec![1].into_iter().find(|n| *n > 5).map(|n| n + 7);
}
