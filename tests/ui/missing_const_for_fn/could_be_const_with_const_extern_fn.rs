#![warn(clippy::missing_const_for_fn)]
#![allow(unsupported_calling_conventions)]
#![feature(const_extern_fn)]

extern "C-unwind" fn c_unwind() {}
//~^ missing_const_for_fn
extern "system" fn system() {}
//~^ missing_const_for_fn
extern "system-unwind" fn system_unwind() {}
//~^ missing_const_for_fn
pub extern "stdcall" fn std_call() {}
//~^ missing_const_for_fn
pub extern "stdcall-unwind" fn std_call_unwind() {}
//~^ missing_const_for_fn
