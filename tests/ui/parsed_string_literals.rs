#![warn(clippy::parsed_string_literals)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn main() {
    _ = "137.194.161.2".parse::<Ipv4Addr>().unwrap();
    //~^ parsed_string_literals

    _ = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    //~^ parsed_string_literals

    _ = "255.255.255.255".parse::<Ipv4Addr>().unwrap();
    //~^ parsed_string_literals

    _ = "0.0.0.0".parse::<Ipv4Addr>().unwrap();
    //~^ parsed_string_literals

    // Wrong address family
    _ = "::1".parse::<Ipv4Addr>().unwrap();
    _ = "127.0.0.1".parse::<Ipv6Addr>().unwrap();

    _ = "::1".parse::<Ipv6Addr>().unwrap();
    //~^ parsed_string_literals

    _ = "::".parse::<Ipv6Addr>().unwrap();
    //~^ parsed_string_literals

    _ = "::1".parse::<IpAddr>().unwrap();
    //~^ parsed_string_literals

    _ = "::".parse::<IpAddr>().unwrap();
    //~^ parsed_string_literals

    // The substition text would be larger than the original and wouldn't use constants
    _ = "2a04:8ec0:0:47::131".parse::<Ipv6Addr>().unwrap();
    _ = "2a04:8ec0:0:47::131".parse::<IpAddr>().unwrap();

    _ = "true".parse::<bool>().unwrap();
    //~^ parsed_string_literals
    _ = "false".parse::<bool>().unwrap();
    //~^ parsed_string_literals

    let _: i64 = "-17".parse().unwrap();
    //~^ parsed_string_literals
    _ = "10".parse::<usize>().unwrap();
    //~^ parsed_string_literals
    _ = "1.23".parse::<f32>().unwrap();
    //~^ parsed_string_literals
    _ = "1.2300".parse::<f32>().unwrap();
    //~^ parsed_string_literals
    _ = "c".parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = r#"""#.parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = "'".parse::<char>().unwrap();
    //~^ parsed_string_literals

    // Check that the original form is preserved ('🦀' == '\u{1f980}')
    _ = "\u{1f980}".parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = "🦀".parse::<char>().unwrap();
    //~^ parsed_string_literals

    // Do not lint invalid values
    _ = "-10".parse::<usize>().unwrap();

    // Do not lint content or code coming from macros
    macro_rules! mac {
        (str) => {
            "10"
        };
        (parse $l:literal) => {
            $l.parse::<u32>().unwrap()
        };
    }
    _ = mac!(str).parse::<u32>().unwrap();
    _ = mac!(parse "10");
}

#[clippy::msrv = "1.29"]
fn msrv_under() {
    _ = "::".parse::<IpAddr>().unwrap();
}
