#![warn(clippy::cast_ptr_sized_int)]

fn main() {
    // Architecture-dependent behavior

    let x: usize = 42;

    // usize to small fixed-size (may truncate on larger ptr widths)
    let _ = x as u8; //~ cast_ptr_sized_int
    let _ = x as u16; //~ cast_ptr_sized_int
    let _ = x as u32; //~ cast_ptr_sized_int
    let _ = x as i8; //~ cast_ptr_sized_int
    let _ = x as i16; //~ cast_ptr_sized_int
    let _ = x as i32; //~ cast_ptr_sized_int

    let y: isize = 42;

    // isize to small fixed-size (may truncate on larger ptr widths)
    let _ = y as u8; //~ cast_ptr_sized_int
    let _ = y as u16; //~ cast_ptr_sized_int
    let _ = y as u32; //~ cast_ptr_sized_int
    let _ = y as i8; //~ cast_ptr_sized_int
    let _ = y as i16; //~ cast_ptr_sized_int
    let _ = y as i32; //~ cast_ptr_sized_int

    // Large fixed-size to ptr-sized (may truncate on smaller ptr widths)
    let c: u32 = 1;
    let d: u64 = 1;
    let e: u128 = 1;
    let _ = c as usize; //~ cast_ptr_sized_int
    let _ = d as usize; //~ cast_ptr_sized_int
    let _ = e as usize; //~ cast_ptr_sized_int

    let h: i32 = 1;
    let i: i64 = 1;
    let j: i128 = 1;
    let _ = h as usize; //~ cast_ptr_sized_int
    let _ = i as usize; //~ cast_ptr_sized_int
    let _ = j as usize; //~ cast_ptr_sized_int

    let _ = c as isize; //~ cast_ptr_sized_int
    let _ = d as isize; //~ cast_ptr_sized_int
    let _ = e as isize; //~ cast_ptr_sized_int
    let _ = h as isize; //~ cast_ptr_sized_int
    let _ = i as isize; //~ cast_ptr_sized_int
    let _ = j as isize; //~ cast_ptr_sized_int

    // usize to signed (potential sign issues)
    let _ = x as i64; //~ cast_ptr_sized_int
}

// Always safe, no architecture dependency

fn no_lint_always_safe() {
    // Small fixed → ptr-sized: always safe (ptr-sized is at least 16-bit)
    let a: u8 = 1;
    let b: u16 = 1;
    let _ = a as usize; // OK: u8 fits in any usize
    let _ = b as usize; // OK: u16 fits in any usize

    let f: i8 = 1;
    let g: i16 = 1;
    let _ = f as isize; // OK: i8 fits in any isize
    let _ = g as isize; // OK: i16 fits in any isize

    // Ptr-sized → large fixed: always safe (ptr-sized is at most 64-bit)
    let x: usize = 42;
    let y: isize = 42;
    let _ = x as u64; // OK: usize fits in u64
    let _ = x as u128; // OK: usize fits in u128
    let _ = y as i64; // OK: isize fits in i64
    let _ = y as i128; // OK: isize fits in i128
}

fn no_lint_same_kind() {
    // Both pointer-sized (handled by other lints)
    let x: usize = 42;
    let _ = x as isize;

    let y: isize = 42;
    let _ = y as usize;

    // Both fixed-size (handled by other lints)
    let a: u32 = 1;
    let _ = a as u64;
    let _ = a as i64;
}
