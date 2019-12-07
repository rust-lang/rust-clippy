#![warn(clippy::as_conversions)]

macro_rules! mcr1 {
    (0_u32 as u64) => {
        ()
    };
}

macro_rules! mcr2 {
    () => {
        0_u32 as u64
    };
}

fn main() {
    let i = 0u32 as u64;

    let j = &i as *const u64 as *mut u64;

    let k = mcr1!(0_u32 as u64);

    let p = mcr2!();
}
