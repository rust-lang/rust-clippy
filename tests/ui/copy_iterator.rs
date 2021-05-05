#![warn(clippy::copy_iterator)]

#[derive(Copy, Clone)]
struct Countdown(u8);

impl Iterator for Countdown {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0.checked_sub(1).map(|c| {
            self.0 = c;
            c
        })
    }
}

fn main() {}
