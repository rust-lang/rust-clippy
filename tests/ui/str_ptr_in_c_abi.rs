#![warn(clippy::str_ptr_in_c_abi)]

use std::ffi::CString;

fn main() {
    // This should use a pointer to a CString
    unsafe { printf("Hello".as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi

    // Like this
    let cstring = CString::new("Hello".as_bytes()).unwrap();
    unsafe { printf(cstring.as_ptr()) };

    // One should also use mut pointers to CStrings
    let mut buffer = String::new();
    let mut buffer = buffer.as_mut_str(); // this lint can only detect `str`s for now
    unsafe { strcpy(buffer.as_mut_ptr() as *mut _, cstring.as_ptr()) };
    //~^ str_ptr_in_c_abi

    let mut cstring_mut = CString::new([]).unwrap();
    unsafe { strcpy(cstring_mut.into_raw(), cstring.as_ptr()) };

    // Two rust strings at once!
    unsafe { strcpy(buffer.as_mut_ptr() as *mut _, "Hello".as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi

    // It can detect smart pointers to str
    let hello_string: String = "Hello".into();
    let hello_box: Box<str> = "Hello".into();
    let hello_rc: std::rc::Rc<str> = "Hello".into();
    unsafe { printf(hello_string.as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi
    unsafe { printf(hello_box.as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi
    unsafe { printf(hello_rc.as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi

    // It detects str to ptr casts in variadics
    let fmt = CString::new("%s\n".as_bytes()).unwrap();
    unsafe { printf(fmt.as_ptr(), "I'm (incorrectly) printf-ing a str!".as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi

    // The cast isn't necessary to trigger the lint, which is relevant for not type-checked variadics
    unsafe { printf(fmt.as_ptr(), "hello".as_ptr()) };
    //~^ str_ptr_in_c_abi

    // It detects the str ptr through multiple casts
    unsafe { printf("hello".as_ptr() as *const _ as *const _) };
    //~^ str_ptr_in_c_abi
    unsafe { printf("hello".as_ptr() as *const _ as *const _ as *const _) };
    //~^ str_ptr_in_c_abi

    // it can see inside of smart pointers to functions
    let printf_box: Box<unsafe extern "C" fn(*const i8, ...) -> i32> = Box::new(printf);
    let printf_rc: std::rc::Rc<unsafe extern "C" fn(*const i8, ...) -> i32> = std::rc::Rc::new(printf);
    unsafe { printf_box("hello".as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi
    unsafe { printf_rc("hello".as_ptr() as *const _) };
    //~^ str_ptr_in_c_abi
    #[allow(clippy::needless_borrow)]
    unsafe {
        (&printf)("hello".as_ptr() as *const _)
        //~^ str_ptr_in_c_abi
    };
}

unsafe extern "C" {
    fn strcpy(dst: *mut i8, src: *const i8) -> *mut i8;
    fn printf(format: *const i8, ...) -> i32;
}
