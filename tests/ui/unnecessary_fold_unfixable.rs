//@no-rustfix
#![warn(clippy::unnecessary_fold)]

/// Folding an `Option` iterator with a non-`Copy` binding as the initial value:
/// substituting the binding into the closure would move it twice, so the lint
/// fires without a suggestion.
fn option_fold_moved_init() {
    let opt: Option<i32> = Some(2);
    let owned = String::from("a");
    let _ = opt.iter().fold(owned, |acc, x| acc + &x.to_string());
    //~^ unnecessary_fold
}

fn main() {}
