//@no-rustfix
fn issue_12670() {
    #[allow(clippy::match_result_ok)]
    let _ = if let Some(x) = "1".parse().ok() {
        x
    } else {
        i32::default()
    };
    let _ = if let Some(x) = None { x } else { i32::default() };
}
