#![warn(clippy::always_true_conditions)]
#[allow(clippy::needless_if)]
fn foo_m(name: &str) {
    if name != "Min" || name != "Max" || name != "Middle" {
        //~^ always_true_conditions
        println!("always prints");
    } else {
        println!("never prints");
    }
    if name != "Min" && name != "Max" {
        println!("condition satisfied");
    } else {
        println!("else");
    }
}

fn foo_s(name: &str) {
    if name != "Min" || name != "Max" {
        //~^ always_true_conditions
        println!("always prints");
    } else {
        println!("never prints");
    }
    if name != "Min" && name != "Max" {
        println!("condition satisfied");
    } else {
        println!("else");
    }
}
