#![warn(clippy::mismatched_target_os)]
#![allow(unused)]

#[cfg(linux)] //~ mismatched_target_os
fn linux() {}

#[cfg(freebsd)] //~ mismatched_target_os
fn freebsd() {}

#[cfg(dragonfly)] //~ mismatched_target_os
fn dragonfly() {}

#[cfg(openbsd)] //~ mismatched_target_os
fn openbsd() {}

#[cfg(netbsd)] //~ mismatched_target_os
fn netbsd() {}

#[cfg(macos)] //~ mismatched_target_os
fn macos() {}

#[cfg(ios)] //~ mismatched_target_os
fn ios() {}

#[cfg(android)] //~ mismatched_target_os
fn android() {}

#[cfg(emscripten)] //~ mismatched_target_os
fn emscripten() {}

#[cfg(fuchsia)] //~ mismatched_target_os
fn fuchsia() {}

#[cfg(haiku)] //~ mismatched_target_os
fn haiku() {}

#[cfg(illumos)] //~ mismatched_target_os
fn illumos() {}

#[cfg(l4re)] //~ mismatched_target_os
fn l4re() {}

#[cfg(redox)] //~ mismatched_target_os
fn redox() {}

#[cfg(solaris)] //~ mismatched_target_os
fn solaris() {}

#[cfg(vxworks)] //~ mismatched_target_os
fn vxworks() {}

// list with conditions
#[cfg(all(not(any(solaris, linux)), freebsd))]
//~^ mismatched_target_os
fn list() {}

// correct use, should be ignored
#[cfg(target_os = "freebsd")]
fn correct() {}

fn main() {}
