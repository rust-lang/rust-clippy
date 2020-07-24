// compile-flags: --test

#![warn(clippy::unwrap_used)]

fn unwrap_option() {
    let opt = Some(0);
    let _ = opt.unwrap();
}

fn unwrap_result() {
    let res: Result<u8, ()> = Ok(0);
    let _ = res.unwrap();
}

#[cfg(test)]
mod test {
    #[test]
    fn test_flag() {
        let opt = Some(0);
        let _ = opt.unwrap();
    }
}

fn main() {}
