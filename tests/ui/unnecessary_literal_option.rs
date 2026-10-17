#![allow(clippy::unnecessary_lazy_evaluations)]
#![warn(clippy::unnecessary_literal_option)]
fn main() {
    let _ = Some(1).is_some();
    //~^ unnecessary_literal_option

    let _ = Some(1).is_none();
    //~^ unnecessary_literal_option

    let _ = None::<i32>.is_some();
    //~^ unnecessary_literal_option

    let _ = None::<i32>.is_none();
    //~^ unnecessary_literal_option

    let _ = Some(1).or(None);
    //~^ unnecessary_literal_option

    let _ = Some(1).or_else(|| None);
    //~^ unnecessary_literal_option

    let _ = None.or(Some(1));

    let x = Some(1);
    let _ = x.is_some();
    //~^ unnecessary_literal_option

    macro_rules! make {
        () => {
            Some(1)
        };
    }
    let z = make!();
    z.is_some();

    macro_rules! m {
        ($x:expr) => {
            $x.is_some()
        };
    }
    let y = Some(1);
    m!(y);

    let _ = None::<i32>.and(Some(1));
    //~^ unnecessary_literal_option

    let _ = None::<i32>.and_then(|_| None::<i32>);
    //~^ unnecessary_literal_option

    let _ = None::<i32>.map(|_| 1);
    //~^ unnecessary_literal_option
}
