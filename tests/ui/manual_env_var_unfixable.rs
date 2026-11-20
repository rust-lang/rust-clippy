//@no-rustfix: the function may have side effects

#![warn(clippy::manual_env_var)]
#![allow(unused)]
use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};

fn might_have_side_effects() -> &'static str {
    "FOO"
}

fn main() {
    // closure can't be switched from lazy to eager implicitly
    let _: Option<String> = env::vars()
        //~^ manual_env_var
        .find(|(k, v)| k == might_have_side_effects())
        .map(|(k, v)| v);
    let _: Option<String> = env::vars().find(|(k, v)| k == might_have_side_effects()).map(|e| e.1);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find(|e| e.0 == might_have_side_effects()).map(|e| e.1);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find_map(|(k, v)| if k == might_have_side_effects() { Some(v) } else { None });
    //~^ manual_env_var
    let _: Option<String> = env::vars().find_map(|(k, v)| if k == might_have_side_effects() { Some(v) } else { None });
    //~^ manual_env_var
    let _: bool = env::vars().any(|(k, v)| k == might_have_side_effects());
    //~^ manual_env_var
    let _: bool = env::vars().any(|(k, v)| might_have_side_effects() == k);
    //~^ manual_env_var
    let _: bool = env::vars().any(|e| e.0 == might_have_side_effects());
    //~^ manual_env_var
    let _: bool = env::vars().any(|e| might_have_side_effects() == e.0);
    //~^ manual_env_var
}
