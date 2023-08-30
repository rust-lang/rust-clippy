#![warn(clippy::all, clippy::redundant_pattern_matching)]
#![allow(unused_must_use)]
#![allow(
    clippy::match_like_matches_macro,
    clippy::needless_bool,
    clippy::needless_if,
    clippy::uninlined_format_args
)]

use std::net::IpAddr::{self, V4, V6};
use std::net::{Ipv4Addr, Ipv6Addr};

fn main() {
    let ipaddr: IpAddr = V4(Ipv4Addr::LOCALHOST);
    if let V4(_) = &ipaddr {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
    //~| NOTE: `-D clippy::redundant-pattern-matching` implied by `-D warnings`

    if let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`

    if let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`

    while let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`

    while let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`

    if V4(Ipv4Addr::LOCALHOST).is_ipv4() {}

    if V6(Ipv6Addr::LOCALHOST).is_ipv6() {}

    if let V4(ipaddr) = V4(Ipv4Addr::LOCALHOST) {
        println!("{}", ipaddr);
    }

    match V4(Ipv4Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
        V4(_) => true,
        V6(_) => false,
    };

    match V4(Ipv4Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`
        V4(_) => false,
        V6(_) => true,
    };

    match V6(Ipv6Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`
        V4(_) => false,
        V6(_) => true,
    };

    match V6(Ipv6Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
        V4(_) => true,
        V6(_) => false,
    };

    let _ = if let V4(_) = V4(Ipv4Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
        true
    } else {
        false
    };

    ipaddr_const();

    let _ = if let V4(_) = gen_ipaddr() {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
        1
    } else if let V6(_) = gen_ipaddr() {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`
        2
    } else {
        3
    };
}

fn gen_ipaddr() -> IpAddr {
    V4(Ipv4Addr::LOCALHOST)
}

const fn ipaddr_const() {
    if let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`

    if let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`

    while let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`

    while let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`

    match V4(Ipv4Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv4()`
        V4(_) => true,
        V6(_) => false,
    };

    match V6(Ipv6Addr::LOCALHOST) {
    //~^ ERROR: redundant pattern matching, consider using `is_ipv6()`
        V4(_) => false,
        V6(_) => true,
    };
}
