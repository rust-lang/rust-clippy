#![warn(clippy::localhost_hardcode)]
#![allow(dead_code)]
#![allow(clippy::identity_op)]
#![allow(clippy::eq_op)]

fn literal_test1() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(255, 255, 255, 255);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(0, 0, 0, 0);
    //~^ localhost_hardcode

    use std::net::Ipv6Addr;
    let _ = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
    //~^ localhost_hardcode
}

fn literal_test2() {
    use std::net;
    let _ = net::Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = net::Ipv4Addr::new(255, 255, 255, 255);
    //~^ localhost_hardcode
    let _ = net::Ipv4Addr::new(0, 0, 0, 0);
    //~^ localhost_hardcode

    let _ = net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
    //~^ localhost_hardcode
}

fn literal_test3() {
    let _ = std::net::Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = std::net::Ipv4Addr::new(255, 255, 255, 255);
    //~^ localhost_hardcode
    let _ = std::net::Ipv4Addr::new(0, 0, 0, 0);
    //~^ localhost_hardcode

    let _ = std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
    //~^ localhost_hardcode
}

const CONST_U8_0: u8 = 0;
const CONST_U8_1: u8 = 1;
const CONST_U8_127: u8 = 127;
const CONST_U8_255: u8 = 255;

const CONST_U16_0: u16 = 0;
const CONST_U16_1: u16 = 1;

fn const_test1() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(CONST_U8_127, CONST_U8_0, CONST_U8_0, CONST_U8_1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(CONST_U8_255, CONST_U8_255, CONST_U8_255, CONST_U8_255);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(CONST_U8_0, CONST_U8_0, CONST_U8_0, CONST_U8_0);
    //~^ localhost_hardcode

    use std::net::Ipv6Addr;
    let _ = Ipv6Addr::new(
        //~^ localhost_hardcode
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_1,
    );

    let _ = Ipv6Addr::new(
        //~^ localhost_hardcode
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
        CONST_U16_0,
    );
}

fn const_test2() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(126 + 1, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(254 + CONST_U8_1, 255, { 255 - CONST_U8_0 }, CONST_U8_255);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(0, CONST_U8_255 - 255, 0, { 1 + 0 - 1 });
    //~^ localhost_hardcode

    use std::net::Ipv6Addr;
    let _ = Ipv6Addr::new(0 + CONST_U16_0, 0, 0, 0, 0, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv6Addr::new(0 + 0, 0, 0, 0, 0, { 2 - 1 - CONST_U16_1 }, 0, 1);
    //~^ localhost_hardcode
}

macro_rules! ipv4_new {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        std::net::Ipv4Addr::new($a, $b, $c, $d)
    };
}

fn macro_test() {
    let _ = ipv4_new!(127, 0, 0, 1);
    // no lint
    let _ = ipv4_new!(255, 255, 255, 255);
    // no lint
    let _ = ipv4_new!(0, 0, 0, 0);
    // no lint
}

fn main() {
    literal_test1();
    literal_test2();
    literal_test3();
    const_test1();
    const_test2();
    macro_test();
}
