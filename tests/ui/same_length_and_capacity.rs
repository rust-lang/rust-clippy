#![feature(vec_into_raw_parts)]
#![warn(clippy::same_length_and_capacity)]

fn main() {
    let mut my_vec: Vec<i32> = Vec::with_capacity(20);
    my_vec.extend([1, 2, 3, 4, 5]);
    let (ptr, mut len, cap) = my_vec.into_raw_parts();
    len = 8;

    let _reconstructed_vec = unsafe { Vec::from_raw_parts(ptr, len, len) };
    //~^ same_length_and_capacity

    // Don't want to lint different expressions for len and cap
    let _properly_reconstructed_vec = unsafe { Vec::from_raw_parts(ptr, len, cap) };

    let my_string = String::from("hello");
    let (string_ptr, string_len, string_cap) = my_string.into_raw_parts();

    let _reconstructed_string = unsafe { String::from_raw_parts(string_ptr, string_len, string_len) };
    //~^ same_length_and_capacity

    // Don't want to lint different expressions for len and cap
    let _properly_reconstructed_string = unsafe { String::from_raw_parts(string_ptr, string_len, string_cap) };
}
