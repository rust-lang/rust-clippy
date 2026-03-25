//@check-pass
// This file has many total lines but only a few code lines.
// Comment lines and blank lines should not be counted.
#![warn(clippy::excessive_file_length)]

// more comments
// to pad the total line count

fn main() {}

fn one() {}

fn two() {}

// even more comments here
// and here
// and here

fn three() {}
