#![warn(clippy::partialeq_to_none)]
#![allow(clippy::eq_op, clippy::needless_if)]

struct Foobar;

impl PartialEq<Option<()>> for Foobar {
    fn eq(&self, _: &Option<()>) -> bool {
        false
    }
}

#[allow(dead_code)]
fn foo(f: Option<u32>) -> &'static str {
    if f != None { "yay" } else { "nay" }
    //~^ ERROR: binary comparison to literal `Option::None`
    //~| NOTE: `-D clippy::partialeq-to-none` implied by `-D warnings`
}

fn foobar() -> Option<()> {
    None
}

fn bar() -> Result<(), ()> {
    Ok(())
}

fn optref() -> &'static &'static Option<()> {
    &&None
}

pub fn macro_expansion() {
    macro_rules! foo {
        () => {
            None::<()>
        };
    }

    let _ = foobar() == foo!();
    let _ = foo!() == foobar();
    let _ = foo!() == foo!();
}

fn main() {
    let x = Some(0);

    let _ = x == None;
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = x != None;
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = None == x;
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = None != x;
    //~^ ERROR: binary comparison to literal `Option::None`

    if foobar() == None {}
    //~^ ERROR: binary comparison to literal `Option::None`

    if bar().ok() != None {}
    //~^ ERROR: binary comparison to literal `Option::None`

    let _ = Some(1 + 2) != None;
    //~^ ERROR: binary comparison to literal `Option::None`

    let _ = { Some(0) } == None;
    //~^ ERROR: binary comparison to literal `Option::None`

    let _ = {
    //~^ ERROR: binary comparison to literal `Option::None`
        /*
          This comment runs long
        */
        Some(1)
    } != None;

    // Should not trigger, as `Foobar` is not an `Option` and has no `is_none`
    let _ = Foobar == None;

    let _ = optref() == &&None;
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = &&None != optref();
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = **optref() == None;
    //~^ ERROR: binary comparison to literal `Option::None`
    let _ = &None != *optref();
    //~^ ERROR: binary comparison to literal `Option::None`

    let x = Box::new(Option::<()>::None);
    let _ = None != *x;
    //~^ ERROR: binary comparison to literal `Option::None`
}
