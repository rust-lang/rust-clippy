#![warn(clippy::clones_into_boxed_slices)]

use std::borrow::ToOwned;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct Dummy {}
impl Display for Dummy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "implements display")
    }
}

macro_rules! create_str {
    ($a:expr, $b:expr) => {
        concat!($a, $b, "!")
    };
}

macro_rules! to_string {
    ($s:expr) => {
        $s.to_string()
    };
}

macro_rules! in_macro {
    ($s:expr) => {
        $s.to_string().into_boxed_str()
        //~^ clones_into_boxed_slices
    };
}

fn main() {
    let s = "test";
    let _: Box<str> = s.to_string().into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = (&s).to_string().into_boxed_str();
    //~^ clones_into_boxed_slices
    let ref_s = &s;
    let _: Box<str> = ref_s.to_string().into_boxed_str();
    //~^ clones_into_boxed_slices
    let boxed_s: Box<str> = Box::from(s);
    let _: Box<str> = (*boxed_s).to_owned().into_boxed_str();
    //~^ clones_into_boxed_slices
    let rc_s: Rc<str> = Rc::from(s);
    let _: Box<str> = (*rc_s).to_owned().into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = s.to_owned().into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = s[..2].to_owned().into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = String::from(s).into_boxed_str();
    //~^ clones_into_boxed_slices
    let string = String::from(s);
    let _: Box<str> = String::from(&string).into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = string.clone().into_boxed_str();
    //~^ clones_into_boxed_slices
    let _: Box<str> = string.to_owned().into_boxed_str();
    //~^ clones_into_boxed_slices

    let c_str = c"test";
    let _: Box<CStr> = c_str.to_owned().into_boxed_c_str();
    //~^ clones_into_boxed_slices
    let c_string = CString::from(c_str);
    let _: Box<CStr> = c_string.clone().into_boxed_c_str();
    //~^ clones_into_boxed_slices
    let _: Box<CStr> = c_string.to_owned().into_boxed_c_str();
    //~^ clones_into_boxed_slices
    let _: Box<CStr> = CString::from(c_str).into_boxed_c_str();
    //~^ clones_into_boxed_slices

    let os_str = OsStr::new("test");
    let _: Box<OsStr> = os_str.to_owned().into_boxed_os_str();
    //~^ clones_into_boxed_slices
    let _: Box<OsStr> = os_str.to_os_string().into_boxed_os_str();
    //~^ clones_into_boxed_slices
    let os_string = OsString::from(os_str);
    let _: Box<OsStr> = os_string.clone().into_boxed_os_str();
    //~^ clones_into_boxed_slices

    let path = Path::new("./");
    let _: Box<Path> = path.to_owned().into_boxed_path();
    //~^ clones_into_boxed_slices
    let _: Box<Path> = path.to_path_buf().into_boxed_path();
    //~^ clones_into_boxed_slices
    let path_buf = PathBuf::from("./");
    let _: Box<Path> = path_buf.clone().into_boxed_path();
    //~^ clones_into_boxed_slices
    let _: Box<Path> = PathBuf::from("./").into_boxed_path();
    //~^ clones_into_boxed_slices

    //Conversions that are necessary and don't clone; don't lint
    let to_os_str = String::from("os_str");
    let _: Box<OsStr> = OsString::from(to_os_str).into_boxed_os_str();
    let to_path = String::from("./");
    let _: Box<Path> = PathBuf::from(to_path).into_boxed_path();

    let test_vec = vec![0u32, 16u32];
    let _: Box<[u32]> = test_vec.clone().into_boxed_slice();
    //~^ clones_into_boxed_slices
    let slice: &[u32] = &test_vec;
    let _: Box<[u32]> = Vec::from(slice).into_boxed_slice();
    //~^ clones_into_boxed_slices
    let _: Box<[u32]> = slice.to_owned().into_boxed_slice();
    //~^ clones_into_boxed_slices
    let _: Box<[u32]> = slice.to_vec().into_boxed_slice();
    //~^ clones_into_boxed_slices

    let _: Box<[u32]> = test_vec.into_boxed_slice();

    //Shouldn't lint because to_string is necessary
    let _: Box<str> = Dummy {}.to_string().into_boxed_str();

    // Do lint when only inner comes from macro
    let _: Box<str> = create_str!("te", "st").to_string().into_boxed_str();
    //~^ clones_into_boxed_slices

    // Don't lint when only part is in macro
    let _: Box<str> = to_string!("test").into_boxed_str();

    // Don't lint here but do lint in the macro def
    let _: Box<str> = in_macro!("test");
}
