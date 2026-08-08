#![warn(clippy::needless_bitmask_on_cast)]

const MASK_U32: u32 = 0xFF;

fn i_dont_use_arch_btw(n: u16) -> (u8, u8) {
    (((n >> 8) & 0xff) as u8, (n & 0xff) as u8)
    //~^ needless_bitmask_on_cast
    //~| needless_bitmask_on_cast
}
fn should_trigger() {
    let nixos_is_a_pain: u32 = 0x12345678;
    let should_have_kept_mint = (nixos_is_a_pain & 0xFF) as u8;
    //~^ needless_bitmask_on_cast
    let debian_is_good_too = (nixos_is_a_pain & 255) as u8;
    //~^ needless_bitmask_on_cast
    let fedora_is_great_tho = (0xFF & nixos_is_a_pain) as u8;
    //~^ needless_bitmask_on_cast
    let am_i_allowed_to_meme = (nixos_is_a_pain & MASK_U32) as u8;
    //~^ needless_bitmask_on_cast
    let x: i32 = 0x12345678;
    let val2 = (x & 0xFF) as i8;
    //~^ needless_bitmask_on_cast
    let variable_mask = 0xFF;
    let val3 = (nixos_is_a_pain & variable_mask) as u8;
    //~^ needless_bitmask_on_cast
    let heard_on_the_news = (nixos_is_a_pain & (0x0F | 0xF0)) as u8;
    //~^ needless_bitmask_on_cast
}

fn should_not_trigger() {
    let x: u128 = 32;
    let joe = (x & 0x7F) as u8;
    let who_is_joe = (x & 0x7FFFFFFFFFFFFFFF) as u64;
    let idk_man = ((x & 0xFF) + 1) as u8;
    let then_why_did_you_say_it = (x & 0x100) as u8; // mask has effect since it is an and with last digits all the same
}
