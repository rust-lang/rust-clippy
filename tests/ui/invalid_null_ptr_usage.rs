// run-rustfix

fn main() {
    let _slice: &[usize] = unsafe { std::slice::from_raw_parts(std::ptr::null(), 0) };
    let _slice: &[usize] = unsafe { std::slice::from_raw_parts(core::ptr::null(), 0) };
}
