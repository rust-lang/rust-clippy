//@ check-pass
#![warn(clippy::undocumented_unsafe_blocks)]

unsafe fn f() {}

fn main() {
    // SAFETY: trust me
    #[allow(
        unused // can be any lint
    )]
    unsafe {
        f();
    }
}
