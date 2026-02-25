#![warn(clippy::assert_multiple)]
#![allow(unused)]

fn main() {
    #[derive(PartialEq)]
    enum Vals {
        Owned,
        Borrowed,
        Other,
    }
    let o = Vals::Owned;
    let b = Vals::Borrowed;
    let other = Vals::Other;

    assert!(o == Vals::Owned && (b != Vals::Borrowed || other == Vals::Other));
    //~^ assert_multiple
}
