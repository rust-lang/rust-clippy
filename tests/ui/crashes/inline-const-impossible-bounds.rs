// `Self` is unsized, so the `where Self: Sized` bound is unsatisfiable and rustc never evaluates
// this body. Clippy must not force the evaluation itself: that would emit `error[E0080]` for code
// which compiles fine. The lint below still fires, since the divisor stays unknown.
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
