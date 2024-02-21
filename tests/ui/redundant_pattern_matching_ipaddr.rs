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
    if let V4(_) = &ipaddr {} //~ redundant_pattern_matching

    if let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    if let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    // Issue 6459
    if matches!(V4(Ipv4Addr::LOCALHOST), V4(_)) {}
    //~^ redundant_pattern_matching

    // Issue 6459
    if matches!(V6(Ipv6Addr::LOCALHOST), V6(_)) {}
    //~^ redundant_pattern_matching

    while let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    while let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    if V4(Ipv4Addr::LOCALHOST).is_ipv4() {}

    if V6(Ipv6Addr::LOCALHOST).is_ipv6() {}

    if let V4(ipaddr) = V4(Ipv4Addr::LOCALHOST) {
        println!("{}", ipaddr);
    }

    //~v redundant_pattern_matching
    match V4(Ipv4Addr::LOCALHOST) {
        V4(_) => true,
        V6(_) => false,
    };

    //~v redundant_pattern_matching
    match V4(Ipv4Addr::LOCALHOST) {
        V4(_) => false,
        V6(_) => true,
    };

    //~v redundant_pattern_matching
    match V6(Ipv6Addr::LOCALHOST) {
        V4(_) => false,
        V6(_) => true,
    };

    //~v redundant_pattern_matching
    match V6(Ipv6Addr::LOCALHOST) {
        V4(_) => true,
        V6(_) => false,
    };

    //~v redundant_pattern_matching
    let _ = if let V4(_) = V4(Ipv4Addr::LOCALHOST) {
        true
    } else {
        false
    };

    ipaddr_const();

    //~v redundant_pattern_matching
    let _ = if let V4(_) = gen_ipaddr() {
        1
    //~v redundant_pattern_matching
    } else if let V6(_) = gen_ipaddr() {
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
    //~^ redundant_pattern_matching

    if let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    while let V4(_) = V4(Ipv4Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    while let V6(_) = V6(Ipv6Addr::LOCALHOST) {}
    //~^ redundant_pattern_matching

    //~v redundant_pattern_matching
    match V4(Ipv4Addr::LOCALHOST) {
        V4(_) => true,
        V6(_) => false,
    };

    //~v redundant_pattern_matching
    match V6(Ipv6Addr::LOCALHOST) {
        V4(_) => false,
        V6(_) => true,
    };
}
