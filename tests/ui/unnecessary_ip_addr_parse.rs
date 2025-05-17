#![warn(clippy::unnecessary_ip_addr_parse)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn main() {
    _ = "137.194.161.2".parse::<Ipv4Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "255.255.255.255".parse::<Ipv4Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "0.0.0.0".parse::<Ipv4Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    // Wrong address family
    _ = "::1".parse::<Ipv4Addr>().unwrap();
    _ = "127.0.0.1".parse::<Ipv6Addr>().unwrap();

    _ = "::1".parse::<Ipv6Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "::".parse::<Ipv6Addr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "::1".parse::<IpAddr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    _ = "::".parse::<IpAddr>().unwrap();
    //~^ unnecessary_ip_addr_parse

    // The substition text would be larger than the original and wouldn't use constants
    _ = "2a04:8ec0:0:47::131".parse::<Ipv6Addr>().unwrap();
    _ = "2a04:8ec0:0:47::131".parse::<IpAddr>().unwrap();
}

#[clippy::msrv = "1.29"]
fn msrv_under() {
    _ = "::".parse::<IpAddr>().unwrap();
}
