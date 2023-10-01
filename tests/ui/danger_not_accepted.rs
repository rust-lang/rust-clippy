#![warn(clippy::danger_not_accepted)]

fn main() {
    wee();
    waz::woo();

    #[clippy::accept_danger(may_deadlock)]
    waz::woo();
}

fn wee() {}

#[clippy::dangerous(may_deadlock)]
mod waz {
    pub fn woo() {}
}
