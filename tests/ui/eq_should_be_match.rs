#![warn(clippy::eq_should_be_match)]

#[derive(PartialEq)]
struct Foo {
    first: u8,
    second: u8,
}

fn main() {
    let d1 = [0, 1];
    let d2 = Foo { first: 0, second: 4 };
    let d3 = (0, 1);

    let _ = d1 == [1, 2];
    //~^ eq_should_be_match
    let _ = d2 == Foo { first: 1, second: 2 };
    //~^ eq_should_be_match
    let _ = d3 != (1, 2);
    //~^ eq_should_be_match
    let _ = d3 == (1, 2) || d3 == (2, 3) || d3 == (1, 4);
    //~^ eq_should_be_match
    // It should only suggest to group the last two items.
    let _ = d2 == Foo { first: 1, second: 2 } || d3 == (2, 3) || d3 == (1, 4);
    //~^ eq_should_be_match
    //~| eq_should_be_match

    let _ = d3 == (2, 3) || d2 == Foo { first: 1, second: 2 } || d3 == (1, 4);
    //~^ eq_should_be_match
    //~| eq_should_be_match
    //~| eq_should_be_match

    let _ = d1 != [1, 2];
    //~^ eq_should_be_match
    let _ = d2 != Foo { first: 1, second: 2 };
    //~^ eq_should_be_match
    let _ = d3 != (1, 2);
    //~^ eq_should_be_match
    let _ = d3 == (1, 2) || d3 != (2, 3) || d3 == (1, 4);
    //~^ eq_should_be_match
    //~| eq_should_be_match
    //~| eq_should_be_match
    let _ = d3 != (1, 2) || d3 == (2, 3) || d3 == (1, 4);
    //~^ eq_should_be_match
    //~| eq_should_be_match
}
