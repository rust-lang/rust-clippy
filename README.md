rust-clippy
===========

A collection of lints that give helpful tips to newbies.

To use clippy on your cargo project:
 1. Clone clippy (`git clone https://github.com/Manishearth/rust-clippy.git`)
 2. Add clippy as a dependency with an explicit path. Sample code for `Cargo.toml`:
    
        [dependencies.clippy]
        path = "/path/to/clippy/"
   
   Place this immediately after your `[dependencies]` section.
 3. Add to your main file code to turn on clippy:

        #![feature(plugin)]
        #![plugin(clippy)]
 
 4. Build, and observe the suggestions.

Be aware that if you use Travis or other autobuild services, your build will not work unless you clone clippy in the Travis commands, as it is not on `crates.io` yet.

Lints included in this crate:

 - `clippy_single_match`: Warns when a match statement with a single nontrivial arm (i.e, where the other arm is `_ => {}`) is used, and recommends `if let` instead.
 - `clippy_box_vec`: Warns on usage of `Box<Vec<T>>`
 - `clippy_dlist`: Warns on usage of `DList`
 - `clippy_str_to_string`: Warns on usage of `str::to_string()`
 - `clippy_toplevel_ref_arg`: Warns when a function argument is declared `ref` (i.e. `fn foo(ref x: u8)`, but not `fn foo((ref x, ref y): (u8, u8))`).

You can allow/warn/deny the whole set using the `clippy` lint group (`#[allow(clippy)]`, etc)


More to come, please [file an issue](https://github.com/Manishearth/rust-clippy/issues) if you have ideas!

Licensed under [MPL](https://www.mozilla.org/MPL/2.0/). If you're having issues with the license, let me know and I'll try to change it to something more permissive.
