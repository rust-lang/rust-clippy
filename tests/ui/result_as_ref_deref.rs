#![allow(unused, clippy::redundant_clone, clippy::useless_vec)]
#![warn(clippy::result_as_ref_deref)]

use std::ffi::{CString, OsString};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

fn main() {
    let mut res = Ok::<_, ()>(String::from("123"));

    let _ = res.clone().as_ref().map(Deref::deref).map(str::len);

    #[rustfmt::skip]
    let _ = res.clone()
        .as_ref().map(
            Deref::deref
        )
        .map(str::len);

    let _ = res.as_mut().map(DerefMut::deref_mut);

    let _ = res.as_ref().map(String::as_str);
    let _ = res.as_ref().map(|x| x.as_str());
    let _ = res.as_mut().map(String::as_mut_str);
    let _ = res.as_mut().map(|x| x.as_mut_str());
    let _ = Ok::<_, ()>(CString::new(vec![]).unwrap())
        .as_ref()
        .map(CString::as_c_str);
    let _ = Ok::<_, ()>(OsString::new()).as_ref().map(OsString::as_os_str);
    let _ = Ok::<_, ()>(PathBuf::new()).as_ref().map(PathBuf::as_path);
    let _ = Ok::<_, ()>(Vec::<()>::new()).as_ref().map(Vec::as_slice);
    let _ = Ok::<_, ()>(Vec::<()>::new()).as_mut().map(Vec::as_mut_slice);

    let _ = res.as_ref().map(|x| x.deref());
    let _ = res.clone().as_mut().map(|x| x.deref_mut()).map(|x| x.len());

    let vc = vec![String::new()];
    let _ = Ok::<_, ()>(1_usize).as_ref().map(|x| vc[*x].as_str()); // should not be linted

    let _: Result<&str, &()> = Ok(&String::new()).as_ref().map(|x| x.as_str()); // should not be linted

    let _ = res.as_ref().map(|x| &**x);
    let _ = res.as_mut().map(|x| &mut **x);

    let _ = res.as_ref().map(std::ops::Deref::deref);
}

#[clippy::msrv = "1.46"]
fn msrv_1_46() {
    let res = Ok::<_, ()>(String::from("123"));
    let _ = res.as_ref().map(String::as_str);
}

#[clippy::msrv = "1.47"]
fn msrv_1_47() {
    let res = Ok::<_, ()>(String::from("123"));
    let _ = res.as_ref().map(String::as_str);
}
