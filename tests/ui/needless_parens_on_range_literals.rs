#![warn(clippy::needless_parens_on_range_literals)]
#![expect(clippy::almost_complete_range)]

fn main() {
    let _ = ('a')..=('z');
    //~^ needless_parens_on_range_literals
    //~| needless_parens_on_range_literals
    let _ = 'a'..('z');
    //~^ needless_parens_on_range_literals

    let _ = (1.)..2.;
    let _ = (1.)..(2.);
    //~^ needless_parens_on_range_literals
    let _ = (1.0)..2.;
    //~^ needless_parens_on_range_literals

    let _ = ('a')..;
    //~^ needless_parens_on_range_literals
    let _ = ..('z');
    //~^ needless_parens_on_range_literals

    macro_rules! zero {
        () => {
            0
        };
    }
    macro_rules! add_paren {
        ($e:expr) => {
            ($e)
        };
    }

    // lint: the paren was added by the user
    let _ = (zero!())..1;
    //~^ needless_parens_on_range_literals
    let _ = 0..(zero!());
    //~^ needless_parens_on_range_literals

    // don't lint: the paren was added by the macro
    let _ = add_paren!(0)..1;
}
