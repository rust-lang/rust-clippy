//@no-rustfix

#![warn(clippy::unnecessary_to_owned)]
use std::borrow::Cow;

fn issue_17842() {
    struct S;

    impl S {
        fn rent_a_cow(&self) -> Cow<'_, Obj> {
            todo!()
        }
    }

    #[derive(Clone)]
    struct Obj;

    fn receiver(_o: &Obj, _s: &mut S) {}

    let mut s = S;
    let cow = s.rent_a_cow();

    receiver(&cow.into_owned(), &mut s);
    //~^ unnecessary_to_owned
}

fn main() {}
