#![feature(cfg_version)]

fn f(i: i32) {
    #[cfg(version("1.50"))]
    let _ = i.isqrt();
    //~^ ERROR: is `1.50.0`

    // When `any/all` are used pick the smallest version seen
    #[cfg(any(version("1.49"), version("1.50")))]
    let _ = i.isqrt();
    //~^ ERROR: is `1.49.0`
    #[cfg(all(version("1.60"), version("1.59")))]
    let _ = i.isqrt();
    //~^ ERROR: is `1.59.0`

    // Ignore negated version requirements
    #[cfg(not(version("1.50")))]
    let _ = i.isqrt();
    #[cfg(not(not(version("1.50"))))]
    let _ = i.isqrt();
    //~^ ERROR: is `1.50.0`
    #[cfg(not(all(version("1.40"), not(version("1.50")))))]
    let _ = i.isqrt();
    //~^ ERROR: is `1.50.0`
}

/// If both are specified on the same node then `clippy::msrv` takes precedence
#[clippy::msrv = "1.50"]
#[cfg(version("1.40"))]
fn both_attributes_cfg_lower(i: i32) {
    let _ = i.isqrt();
    //~^ ERROR: is `1.50.0`
}

#[clippy::msrv = "1.40"]
#[cfg(version("1.50"))]
fn both_attributes_cfg_higher(i: i32) {
    let _ = i.isqrt();
    //~^ ERROR: is `1.40.0`
}
