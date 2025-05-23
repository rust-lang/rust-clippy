#![feature(custom_inner_attributes)]
#![clippy::msrv = "1.15"]

fn main() {
    // Should not suggest unwrap_or_default() when MSRV is 1.15
    let x: Option<Vec<u32>> = Some(vec![1, 2, 3]);
    #[allow(clippy::manual_unwrap_or)]
    let _ = match x {
        Some(v) => v,
        None => vec![],
    };

    // Should not suggest unwrap_or_default() for Result when MSRV is 1.15
    let y: Result<Vec<u32>, &str> = Ok(vec![1, 2, 3]);
    #[allow(clippy::manual_unwrap_or)]
    let _ = match y {
        Ok(v) => v,
        Err(_) => vec![],
    };
}

// Test with MSRV 1.16
#[clippy::msrv = "1.16"]
fn msrv_1_16() {
    // Should suggest unwrap_or_default() when MSRV is 1.16
    let x: Option<Vec<u32>> = Some(vec![1, 2, 3]);
    let _ = match x {
        Some(v) => v,
        None => vec![],
    };

    // Should suggest unwrap_or_default() for Result when MSRV is 1.16
    let y: Result<Vec<u32>, &str> = Ok(vec![1, 2, 3]);
    let _ = match y {
        Ok(v) => v,
        Err(_) => vec![],
    };
}
