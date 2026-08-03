// Evaluating an inline const block inside an item whose bounds can't be satisfied must not report
// the evaluation failure, as the compiler never evaluates that body itself.
#![feature(trivial_bounds)]
#![warn(clippy::arithmetic_side_effects)]

struct UnreachableConst(str);

impl UnreachableConst {
    fn unreachable(f: u64) -> u64
    where
        Self: Sized,
    {
        f.saturating_div(const { size_of::<Self>() as u64 })
        //~^ arithmetic_side_effects
    }
}

fn main() {}
