#![warn(clippy::danger_not_accepted)]

fn main() {
    wee();
    waz::woo();

    #[clippy::accept_danger(may_deadlock)]
    waz::woo();

    Maz.faz();

    #[clippy::accept_danger(may_deadlock)]
    Maz.faz();

    #[clippy::accept_danger(may_deadlock, may_delete_system)]
    Maz.faz();

    Maz.faz2();

    #[clippy::accept_danger(may_deadlock)]
    Maz.faz2();

    #[clippy::accept_danger(may_deadlock, may_delete_system)]
    Maz.faz2();

    waz::woo2();

    #[clippy::accept_danger(may_deadlock)]
    waz::woo2();
}

fn wee() {}

struct Maz;

#[clippy::dangerous(may_deadlock)]
mod waz {
    pub fn woo() {}

    #[clippy::dangerous(may_deadlock)]
    pub fn woo2() {}

    impl super::Maz {
        #[clippy::dangerous(may_delete_system)]
        pub fn faz(&self) {}
    }

    impl super::FazTrait for super::Maz {
        fn faz2(&self) {}
    }
}

trait FazTrait {
    #[clippy::dangerous(may_delete_system)]
    fn faz2(&self);
}
