#![warn(clippy::option_map_or_err_ok)]

fn main() {
    let x = Some("a");
    let _ = x.map_or(Err("a"), Ok);
    //~^ ERROR: called `map_or(Err(_), Ok)` on an `Option` value
    let x = &Some("a");
    let _ = x.map_or(Err("a"), Ok);

    #[expect(clippy::redundant_closure)]
    {
        // Do not lint because the `redundant-closure` lint will be emitted on
        // the closure already.
        let _ = x.map_or(Err("a"), |x| Ok(x));
    }
    #[allow(clippy::redundant_closure)]
    {
        let _ = x.map_or(Err("a"), |x| Ok(x));
        //~^ ERROR: called `map_or(Err(_), Ok)` on an `Option` value
    }
}
