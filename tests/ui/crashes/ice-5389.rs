#![allow(clippy::explicit_counter_loop, clippy::let_arr_const)]

fn main() {
    let v = vec![1, 2, 3];
    let mut i = 0;
    let max_storage_size = [0; 128 * 1024];
    for item in &v {
        bar(i, *item);
        i += 1;
    }
}

fn bar(_: usize, _: u32) {}
