#![warn(clippy::unnecessary_self_imports)]
#![allow(unused_imports, dead_code)]

use std::collections::hash_map::{self, *};
use std::fs::{self as alias};
//~^ ERROR: import ending with `::{self}`
//~| NOTE: this will slightly change semantics; any non-module items at the same path will
use std::io::{self, Read};
use std::rc::{self};
//~^ ERROR: import ending with `::{self}`
//~| NOTE: this will slightly change semantics; any non-module items at the same path will

fn main() {}
