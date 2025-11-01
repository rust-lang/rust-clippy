#![allow(clippy::inconsistent_digit_grouping)]

fn main() {
    let arr = [b'a', b'b', b'c'];
    let ptr = arr.as_ptr();

    let var = 32;
    const CONST: isize = 42;

    unsafe {
        let _ = ptr.offset(0);
        //~^ ptr_offset_by_literal
        let _ = ptr.offset(-0);
        //~^ ptr_offset_by_literal

        let _ = ptr.offset(5);
        //~^ ptr_offset_by_literal
        let _ = ptr.offset(-5);
        //~^ ptr_offset_by_literal

        let _ = ptr.offset(var);
        let _ = ptr.offset(CONST);

        let _ = ptr.wrapping_offset(5isize);
        //~^ ptr_offset_by_literal
        let _ = ptr.wrapping_offset(-5isize);
        //~^ ptr_offset_by_literal

        let _ = ptr.offset(-(5));
        //~^ ptr_offset_by_literal
        let _ = ptr.wrapping_offset(-(5));
        //~^ ptr_offset_by_literal

        // isize::MAX and isize::MIN on 64-bit systems.
        let _ = ptr.offset(9_223_372_036_854_775_807isize);
        //~^ ptr_offset_by_literal
        let _ = ptr.offset(-9_223_372_036_854_775_808isize);
        //~^ ptr_offset_by_literal

        let _ = ptr.offset(5_0__isize);
        //~^ ptr_offset_by_literal
        let _ = ptr.offset(-5_0__isize);
        //~^ ptr_offset_by_literal

        macro_rules! offs { { $e:expr, $offs:expr } => { $e.offset($offs) }; }
        offs!(ptr, 6);
        offs!(ptr, var);
    }
}
