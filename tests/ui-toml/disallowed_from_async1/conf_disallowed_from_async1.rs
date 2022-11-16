#![warn(clippy::disallowed_from_async)]

pub async fn call_from_async() -> Vec<u64> {
    let mut v = Vec::new();
    while v.len() < 3 {
        v.push(2);
    }
    v
}

pub async fn call_from_closure_in_async_indirect() -> Vec<u64> {
    let f = |()| {
        let mut v = Vec::<u64>::new();
        while v.len() < 3 {
            v.push(2);
        }
        v
    };
    std::iter::repeat(()).take(5).flat_map(f).collect()
}

fn main() {}
