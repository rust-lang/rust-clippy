//! Test casts for alignment issues

#![feature(core_intrinsics)]
#![feature(pointer_is_aligned_to)]
#![warn(clippy::cast_ptr_alignment)]
#![expect(clippy::no_effect)]

fn main() {
    /* These should be warned against */

    // cast to more-strictly-aligned type
    (&1u8 as *const u8) as *const u16;
    //~^ cast_ptr_alignment

    (&mut 1u8 as *mut u8) as *mut u16;
    //~^ cast_ptr_alignment

    // cast to more-strictly-aligned type, but with the `pointer::cast` function.
    (&1u8 as *const u8).cast::<u16>();
    //~^ cast_ptr_alignment

    (&mut 1u8 as *mut u8).cast::<u16>();
    //~^ cast_ptr_alignment

    /* These should be ok */

    // not a pointer type
    1u8 as u16;
    // cast to less-strictly-aligned type
    (&1u16 as *const u16) as *const u8;
    (&mut 1u16 as *mut u16) as *mut u8;
    // For c_void, we should trust the user. See #2677
    (&1u32 as *const u32 as *const std::os::raw::c_void) as *const u32;
    (&1u32 as *const u32 as *const libc::c_void) as *const u32;
    // For ZST, we should trust the user. See #4256
    (&1u32 as *const u32 as *const ()) as *const u32;

    // Issue #2881
    let mut data = [0u8, 0u8];
    unsafe {
        let ptr = &data as *const [u8; 2] as *const u8;
        let _ = (ptr as *const u16).read_unaligned();
        let _ = core::ptr::read_unaligned(ptr as *const u16);
        let _ = core::intrinsics::unaligned_volatile_load(ptr as *const u16);
        let ptr = &mut data as *mut [u8; 2] as *mut u8;
        (ptr as *mut u16).write_unaligned(0);
        core::ptr::write_unaligned(ptr as *mut u16, 0);
        core::intrinsics::unaligned_volatile_store(ptr as *mut u16, 0);
    }
}

// alignment check that follows after creating the pointer, but before use
mod issue17636 {
    /* Checking the alignment of the cast itself */

    fn checked_immediately() {
        let _ = ((&1u8 as *const u8) as *const u16).is_aligned();
        let _ = (&1u8 as *const u8).cast::<u16>().is_aligned();
        let _ = ((&mut 1u8 as *mut u8) as *mut u16).is_aligned();
        let _ = (&mut 1u8 as *mut u8).cast::<u16>().is_aligned();
    }

    /* The guard's branch must diverge */

    fn guard_diverges() {
        let ptr = (&1u8 as *const u8).cast::<u16>();
        if !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn guard_does_not_diverge() {
        let ptr = (&1u8 as *const u8).cast::<u16>();
        //~^ cast_ptr_alignment
        if !ptr.is_aligned() {
            let _ = ();
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn diverges_on_aligned() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    // This guard establishes alignment, but `else` branches are not recognized.
    fn guard_with_else_branch() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if !ptr.is_aligned() {
            return;
        } else {
            unsafe {
                let _ = ptr.read();
            }
        }

        std::hint::black_box(ptr);
    }

    /* The check must come before every other use */

    fn guard_after_unrelated_statement() {
        let data = [0_u8; 2];
        let ptr = data.as_ptr().cast::<u16>();
        let len = data.len();
        if !ptr.is_aligned() || len < size_of::<u16>() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn used_before_guard() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        let _ = ptr;
        if !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn used_in_rejecting_branch() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if !ptr.is_aligned() {
            unsafe { ptr.read() };
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    /* The guard must sit in the pointer's own block */

    #[expect(clippy::collapsible_if)]
    fn guard_inside_conditional_block(flag: bool) {
        let data = [0_u8; 4];
        let ptr = data.as_ptr().cast::<u16>();
        //~^ cast_ptr_alignment
        if flag {
            if !ptr.is_aligned() {
                return;
            }
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn guard_inside_closure() {
        let data = [0_u8; 4];
        let ptr = data.as_ptr().cast::<u16>();
        //~^ cast_ptr_alignment
        let check = || {
            if !ptr.is_aligned() {
                return;
            }
            std::hint::black_box(());
        };
        check();

        unsafe {
            let _ = ptr.read();
        }
    }

    fn guard_inside_loop(n: usize) {
        let data = [0_u8; 4];
        let ptr = data.as_ptr().cast::<u16>();
        //~^ cast_ptr_alignment
        for _ in 0..n {
            if !ptr.is_aligned() {
                break;
            }
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    /* `!ptr.is_aligned()` as one operand of a `||` chain */

    fn guard_is_first_operand() {
        let data = [0_u8; 2];
        let ptr = data.as_ptr() as *const u16;
        if !ptr.is_aligned() || data.len() < size_of::<u16>() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn guard_is_last_operand() {
        let data = [0_u8; 2];
        let ptr = data.as_ptr() as *const u16;
        if data.len() < size_of::<u16>() || !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn unrelated_operand_is_negated(flag: bool) {
        let data = [0_u8; 2];
        let ptr = data.as_ptr() as *const u16;
        if !flag || !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn used_after_guard_operand() {
        let data = [0_u8; 2];
        let ptr = data.as_ptr() as *const u16;
        if !ptr.is_aligned() || unsafe { ptr.read() } == 0 {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn read_before_guard_operand() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if unsafe { ptr.read() } == 0 || !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn guard_in_and_chain(flag: bool) {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if flag && !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    /* The guard must be about this pointer's alignment */

    fn guard_on_a_different_pointer() {
        let data = [0_u8; 4];
        let checked = data.as_ptr().cast::<u16>();
        let unchecked = data[2..].as_ptr().cast::<u16>();
        //~^ cast_ptr_alignment
        if !checked.is_aligned() {
            return;
        }

        unsafe {
            let _ = unchecked.read();
        }
    }

    fn unrelated_diverging_guard() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if std::hint::black_box(false) {
            let _ = ptr;
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn address_arithmetic_check() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if !ptr.addr().is_multiple_of(align_of::<u16>()) {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn is_aligned_to_check() {
        let ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if !ptr.is_aligned_to(align_of::<u16>()) {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    /* Only a plain, immutable binding of the cast is tracked */

    fn mutable_binding() {
        let mut ptr = (&1u8 as *const u8) as *const u16;
        //~^ cast_ptr_alignment
        if !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn destructured_binding() {
        let (ptr,) = (((&1u8 as *const u8) as *const u16),);
        //~^ cast_ptr_alignment
        if !ptr.is_aligned() {
            return;
        }

        unsafe {
            let _ = ptr.read();
        }
    }

    fn mutable_pointer() {
        let mut data = [0_u8; 2];
        let ptr = data.as_mut_ptr().cast::<u16>();
        if !ptr.is_aligned() {
            return;
        }

        unsafe {
            ptr.write(0);
        }
    }
}
