#![warn(clippy::mismatched_target_os)]
#![allow(unused)]

#[cfg(linux)]
//~^ ERROR: operating system used in target family position
fn linux() {}

#[cfg(freebsd)]
//~^ ERROR: operating system used in target family position
fn freebsd() {}

#[cfg(dragonfly)]
//~^ ERROR: operating system used in target family position
fn dragonfly() {}

#[cfg(openbsd)]
//~^ ERROR: operating system used in target family position
fn openbsd() {}

#[cfg(netbsd)]
//~^ ERROR: operating system used in target family position
fn netbsd() {}

#[cfg(macos)]
//~^ ERROR: operating system used in target family position
fn macos() {}

#[cfg(ios)]
//~^ ERROR: operating system used in target family position
fn ios() {}

#[cfg(android)]
//~^ ERROR: operating system used in target family position
fn android() {}

#[cfg(emscripten)]
//~^ ERROR: operating system used in target family position
fn emscripten() {}

#[cfg(fuchsia)]
//~^ ERROR: operating system used in target family position
fn fuchsia() {}

#[cfg(haiku)]
//~^ ERROR: operating system used in target family position
fn haiku() {}

#[cfg(illumos)]
//~^ ERROR: operating system used in target family position
fn illumos() {}

#[cfg(l4re)]
//~^ ERROR: operating system used in target family position
fn l4re() {}

#[cfg(redox)]
//~^ ERROR: operating system used in target family position
fn redox() {}

#[cfg(solaris)]
//~^ ERROR: operating system used in target family position
fn solaris() {}

#[cfg(vxworks)]
//~^ ERROR: operating system used in target family position
fn vxworks() {}

// list with conditions
#[cfg(all(not(any(solaris, linux)), freebsd))]
//~^ ERROR: operating system used in target family position
fn list() {}

// correct use, should be ignored
#[cfg(target_os = "freebsd")]
fn correct() {}

fn main() {}
