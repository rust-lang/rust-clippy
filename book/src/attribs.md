# Attributes for Crate Authors

In some cases it is possible to extend Clippy coverage to 3rd party libraries.
To do this, Clippy provides attributes that can be applied to items in the 3rd party crate.

## `#[clippy::format_args]`

_Available since Clippy v1.85_

This attribute can be added to a macro that supports `format!`, `println!`, or similar syntax.
It tells Clippy that the macro is a formatting macro, and that the arguments to the macro
should be linted as if they were arguments to `format!`. Any lint that would apply to a
`format!` call will also apply to the macro call. The macro may have additional arguments
before the format string, and these will be ignored.

### Example

```rust
/// A macro that prints a message if a condition is true.
#[macro_export]
#[clippy::format_args]
macro_rules! print_if {
    ($condition:expr, $($args:tt)+) => {{
        if $condition {
            println!($($args)+)
        }
    }};
}
```

## `#[clippy::has_significant_drop]`

_Available since Clippy v1.60_

The `clippy::has_significant_drop` attribute can be added to types whose Drop impls have an important side effect,
such as unlocking a mutex, making it important for users to be able to accurately understand their lifetimes.
When a temporary is returned in a function call in a match scrutinee, its lifetime lasts until the end of the match
block, which may be surprising.

### Example

```rust
#[clippy::has_significant_drop]
struct CounterWrapper<'a> {
    counter: &'a Counter,
}

impl<'a> Drop for CounterWrapper<'a> {
    fn drop(&mut self) {
        self.counter.i.fetch_sub(1, Ordering::Relaxed);
    }
}
```

## `#[clippy::private_interior_mutability]`

_Available since Clippy v1.100_

The `clippy::private_interior_mutability` attribute can be added to types which
internally contain interior-mutable types, but don't expose this fact in their
API. This is identical to the `ignore-interior-mutability` configuration option,
but can be placed on the offending type instead of in Clippy's configuration file.

### Example

```rust
use std::{cell::Cell, collections::HashMap};

#[clippy::private_interior_mutability]
struct ActuallyFineAsAKey {
    id: usize,
    other_data: Cell<i32>,
}

fn main() {
    // `mut_key` will not fire here, as `ActuallyFineAsAKey` is considered to
    // lack interior mutability
    let _map: HashMap<ActuallyFineAsAKey, String> = HashMap::new();
}
```
