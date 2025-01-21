#![warn(clippy::return_and_then)]

fn main() {
    fn test_opt_block(opt: Option<i32>) -> Option<i32> {
        opt.and_then(|n| {
            let mut ret = n + 1;
            ret += n;
            if n > 1 { Some(ret) } else { None }
        })
    }

    fn test_opt_func(opt: Option<i32>) -> Option<i32> {
        opt.and_then(|n| test_opt_block(Some(n)))
    }

    fn test_call_chain() -> Option<i32> {
        gen_option(1).and_then(|n| test_opt_func(Some(n)))
    }

    fn test_res_block(opt: Result<i32, i32>) -> Result<i32, i32> {
        opt.and_then(|n| if n > 1 { Ok(n + 1) } else { Err(n) })
    }

    fn test_res_func(opt: Result<i32, i32>) -> Result<i32, i32> {
        opt.and_then(|n| test_res_block(Ok(n)))
    }
}

fn gen_option(n: i32) -> Option<i32> {
    Some(n)
}
