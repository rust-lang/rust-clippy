// run-rustfix
#![warn(clippy::manual_map)]

use std::panic::Location;

#[track_caller]
fn caller_line() -> u32 {
    Location::caller().line()
}

fn make_value(value: u32, caller: u32) -> (u32, u32) {
    (value, caller)
}

#[track_caller]
fn track_caller_false_positive(value: Option<u32>) -> Option<(u32, u32)> {
    match value {
        Some(value) => Some(make_value(value, caller_line())),
        None => None,
    }
}

#[track_caller]
fn tracked_function(value: u32) -> u32 {
    let _ = Location::caller();
    value
}

#[track_caller]
fn track_caller_function_item(value: Option<u32>) -> Option<u32> {
    match value {
        Some(value) => Some(tracked_function(value)),
        None => None,
    }
}

#[track_caller]
fn track_caller_without_tracked_call(value: Option<u32>) -> Option<u32> {
    match value {
        //~^ manual_map
        Some(value) => Some(value + 1),
        None => None,
    }
}

fn untracked_with_tracked_call(value: Option<u32>) -> Option<(u32, u32)> {
    match value {
        //~^ manual_map
        Some(value) => Some(make_value(value, caller_line())),
        None => None,
    }
}

trait TrackCallerMap {
    #[track_caller]
    fn map(value: Option<u32>) -> Option<(u32, u32)>;
}

impl TrackCallerMap for () {
    fn map(value: Option<u32>) -> Option<(u32, u32)> {
        match value {
            Some(value) => Some(make_value(value, caller_line())),
            None => None,
        }
    }
}

fn main() {}
