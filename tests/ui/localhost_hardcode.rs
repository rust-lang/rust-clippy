#![warn(clippy::localhost_hardcode)]
#![allow(dead_code)]

fn literal_test1() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
}

fn literal_test2() {
    use std::net;
    let _ = net::Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
}

fn literal_test3() {
    let _ = std::net::Ipv4Addr::new(127, 0, 0, 1);
    //~^ localhost_hardcode
}

const CONSTANT_1: u8 = 127;
const CONSTANT_2: u8 = 0;
const CONSTANT_3: u8 = 0;
const CONSTANT_4: u8 = 1;

fn const_test1() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(CONSTANT_1, CONSTANT_2, CONSTANT_3, CONSTANT_4);
    //~^ localhost_hardcode
}

fn const_test2() {
    use std::net;
    let _ = net::Ipv4Addr::new(CONSTANT_1, CONSTANT_2, CONSTANT_3, CONSTANT_4);
    //~^ localhost_hardcode
}

fn const_test3() {
    let _ = std::net::Ipv4Addr::new(CONSTANT_1, CONSTANT_2, CONSTANT_3, CONSTANT_4);
    //~^ localhost_hardcode
}

fn const_test4() {
    use std::net::Ipv4Addr;
    let _ = Ipv4Addr::new(126 + 1, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(126 + CONSTANT_4, 0, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new(127, 2 - 1 - CONSTANT_4, 0, 1);
    //~^ localhost_hardcode
    let _ = Ipv4Addr::new({ 127 }, 0, 0, 1);
    //~^ localhost_hardcode
}

fn main() {
    literal_test1();
    literal_test2();
    literal_test3();
    const_test1();
    const_test2();
    const_test3();
    const_test4();
}
