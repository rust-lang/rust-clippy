//@only-target-x86_64
//@ignore-target-aarch64

#[warn(clippy::inline_asm_x86_intel_syntax)]
mod warn_intel {
    pub(super) unsafe fn use_asm() {
        use std::arch::asm;
        asm!("");
        //~^ ERROR: Intel x86 assembly syntax used
        asm!("", options());
        //~^ ERROR: Intel x86 assembly syntax used
        asm!("", options(nostack));
        //~^ ERROR: Intel x86 assembly syntax used
        asm!("", options(att_syntax));
        asm!("", options(nostack, att_syntax));
    }
}

#[warn(clippy::inline_asm_x86_att_syntax)]
mod warn_att {
    pub(super) unsafe fn use_asm() {
        use std::arch::asm;
        asm!("");
        asm!("", options());
        asm!("", options(nostack));
        asm!("", options(att_syntax));
        //~^ ERROR: AT&T x86 assembly syntax used
        asm!("", options(nostack, att_syntax));
        //~^ ERROR: AT&T x86 assembly syntax used
    }
}

#[cfg(target_arch = "x86_64")]
fn main() {
    unsafe {
        warn_att::use_asm();
        warn_intel::use_asm();
    }
}
