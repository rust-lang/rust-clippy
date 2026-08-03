//@no-rustfix: the suggestion would remove the comment before `.get()`

#![warn(clippy::unnecessary_nonzero_get)]

use std::num::NonZero;

fn main() {
    let nz = NonZero::new(1u32).unwrap();

    // This comment must not be removed by an automatic fix.
    let _ = nz /* keep this comment */
        .get()
        //~^ unnecessary_nonzero_get
        .leading_zeros();
}
