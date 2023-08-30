//@edition:2018

#![warn(clippy::needless_parens_on_range_literals)]
#![allow(clippy::almost_complete_range)]

fn main() {
    let _ = ('a')..=('z');
    //~^ ERROR: needless parenthesis on range literals can be removed
    //~| NOTE: `-D clippy::needless-parens-on-range-literals` implied by `-D warnings`
    //~| ERROR: needless parenthesis on range literals can be removed
    let _ = 'a'..('z');
    //~^ ERROR: needless parenthesis on range literals can be removed
    let _ = (1.)..2.;
    let _ = (1.)..(2.);
    //~^ ERROR: needless parenthesis on range literals can be removed
    let _ = ('a')..;
    //~^ ERROR: needless parenthesis on range literals can be removed
    let _ = ..('z');
    //~^ ERROR: needless parenthesis on range literals can be removed
}
