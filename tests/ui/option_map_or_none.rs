#![allow(clippy::bind_instead_of_map)]

fn main() {
    let opt = Some(1);
    let r: Result<i32, i32> = Ok(1);
    let bar = |_| Some(1);

    // Check `OPTION_MAP_OR_NONE`.
    // Single line case.
    let _: Option<i32> = opt.map_or(None, |x| Some(x + 1));
    //~^ ERROR: called `map_or(None, ..)` on an `Option` value. This can be done more dire
    //~| NOTE: `-D clippy::option-map-or-none` implied by `-D warnings`
    // Multi-line case.
    #[rustfmt::skip]
    let _: Option<i32> = opt.map_or(None, |x| {
    //~^ ERROR: called `map_or(None, ..)` on an `Option` value. This can be done more dire
                        Some(x + 1)
                       });
    // function returning `Option`
    let _: Option<i32> = opt.map_or(None, bar);
    //~^ ERROR: called `map_or(None, ..)` on an `Option` value. This can be done more dire
    let _: Option<i32> = opt.map_or(None, |x| {
    //~^ ERROR: called `map_or(None, ..)` on an `Option` value. This can be done more dire
        let offset = 0;
        let height = x;
        Some(offset + height)
    });

    // Check `RESULT_MAP_OR_INTO_OPTION`.
    let _: Option<i32> = r.map_or(None, Some);
    //~^ ERROR: called `map_or(None, Some)` on a `Result` value. This can be done more dir
    //~| NOTE: `-D clippy::result-map-or-into-option` implied by `-D warnings`
}
