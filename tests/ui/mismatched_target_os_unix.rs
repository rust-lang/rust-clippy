// run-rustfix

#![warn(clippy::mismatched_target_os)]
#![allow(unused)]

#[cfg(linux)]
fn linux() {}

#[cfg(freebsd)]
fn freebsd() {}

#[cfg(dragonfly)]
fn dragonfly() {}

#[cfg(openbsd)]
fn openbsd() {}

#[cfg(netbsd)]
fn netbsd() {}

#[cfg(macos)]
fn macos() {}

#[cfg(ios)]
fn ios() {}

#[cfg(android)]
fn android() {}

#[cfg(emscripten)]
fn emscripten() {}

#[cfg(fuchsia)]
fn fuchsia() {}

#[cfg(haiku)]
fn haiku() {}

#[cfg(illumos)]
fn illumos() {}

#[cfg(l4re)]
fn l4re() {}

#[cfg(redox)]
fn redox() {}

#[cfg(solaris)]
fn solaris() {}

#[cfg(vxworks)]
fn vxworks() {}

// list with conditions
#[cfg(all(not(any(solaris, linux)), freebsd))]
fn list() {}

// correct use, should be ignored
#[cfg(target_os = "freebsd")]
fn correct() {}

fn macro_use() {
    if cfg!(android) {}
    if cfg!(dragonfly) {}
    if cfg!(emscripten) {}
    if cfg!(freebsd) {}
    if cfg!(fuchsia) {}
    if cfg!(haiku) {}
    if cfg!(illumos) {}
    if cfg!(ios) {}
    if cfg!(l4re) {}
    if cfg!(linux) {}
    if cfg!(macos) {}
    if cfg!(netbsd) {}
    if cfg!(openbsd) {}
    if cfg!(redox) {}
    if cfg!(solaris) {}
    if cfg!(vxworks) {}
    if cfg!(hermit) {}
    if cfg!(none) {}
    if cfg!(wasi) {}
    if cfg!(any(linux, macos)) {}
    if cfg!(all(not(any(solaris, linux)), freebsd)) {}

    // correct use
    if cfg!(target_os = "macos") {}
}

fn main() {}
