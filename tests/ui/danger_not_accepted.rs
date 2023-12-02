#![warn(clippy::danger_not_accepted)]

fn main() {
    wee();
    waz::woo();

    #[clippy::accept_danger(may_deadlock)]
    waz::woo();

    Maz.faz();

    #[clippy::accept_danger(may_deadlock, reason = "this is fine :)")]
    Maz.faz();

    #[clippy::accept_danger(may_deadlock, not_a_virus::may_delete_system)]
    Maz.faz();

    Maz.faz2();

    #[clippy::accept_danger(may_deadlock)]
    Maz.faz2();

    #[clippy::accept_danger(may_deadlock, not_a_virus::may_delete_system)]
    Maz.faz2();

    waz::woo2();

    #[clippy::accept_danger(may_deadlock)]
    waz::woo2();
}

fn wee() {}

struct Maz;

#[clippy::dangerous(may_deadlock = "this entire module is just really messed up")]
mod waz {
    pub fn woo() {}

    #[clippy::dangerous(may_deadlock = "your program may deadlock in calling this function")]
    pub fn woo2() {}

    impl super::Maz {
        #[clippy::dangerous(
            not_a_virus::may_delete_system = "calling this has a very strong chance of just deleting your computer"
        )]
        pub fn faz(&self) {}
    }

    impl super::FazTrait for super::Maz {
        fn faz2(&self) {}
    }
}

trait FazTrait {
    #[clippy::dangerous(not_a_virus::may_delete_system = "this is a justification")]
    fn faz2(&self);
}

// Edge case attr tests
#[rustfmt::skip]
#[clippy::dangerous(whee, woo,)]
#[clippy::dangerous(whee, sdjfkl::woo, reason = "sdfhsdf",)]
#[clippy::accept_danger(hehe::haha = "sjdfkljf",)]
fn dummy_1() {}

// Invalid attr tests
#[clippy::dangerous{}]
#[clippy::dangerous[]]
#[clippy::dangerous(,)]
#[clippy::dangerous(whee, reason)]
#[clippy::dangerous(whee, reason, abc)]
#[clippy::dangerous(whee, reason =)]
#[clippy::dangerous(whee, reason =, weh)]
#[clippy::dangerous(whee, reason = "", weh)]
#[clippy::dangerous(whee, reason = "" weh)]
#[clippy::dangerous(whee, reason = 4)]
#[clippy::dangerous(unsafe::bar,, ehe = "dhf", bar::unsafe == "hehe")]
#[clippy::dangerous(crate::bar, reason)]
#[clippy::accept_danger(clippy:: = "" ::, ::)]
#[clippy::accept_danger(clippy::boo = "")]
fn dummy_2() {}
