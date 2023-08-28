#![allow(unused, clippy::redundant_clone, clippy::useless_vec)]
#![warn(clippy::option_as_ref_deref)]

use std::ffi::{CString, OsString};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

fn main() {
    let mut opt = Some(String::from("123"));

    let _ = opt.clone().as_ref().map(Deref::deref).map(str::len);
    //~^ ERROR: called `.as_ref().map(Deref::deref)` on an Option value. This can be done
    //~| NOTE: `-D clippy::option-as-ref-deref` implied by `-D warnings`

    #[rustfmt::skip]
    let _ = opt.clone()
    //~^ ERROR: called `.as_ref().map(Deref::deref)` on an Option value. This can be done
        .as_ref().map(
            Deref::deref
        )
        .map(str::len);

    let _ = opt.as_mut().map(DerefMut::deref_mut);
    //~^ ERROR: called `.as_mut().map(DerefMut::deref_mut)` on an Option value. This can b

    let _ = opt.as_ref().map(String::as_str);
    //~^ ERROR: called `.as_ref().map(String::as_str)` on an Option value. This can be don
    let _ = opt.as_ref().map(|x| x.as_str());
    //~^ ERROR: called `.as_ref().map(|x| x.as_str())` on an Option value. This can be don
    let _ = opt.as_mut().map(String::as_mut_str);
    //~^ ERROR: called `.as_mut().map(String::as_mut_str)` on an Option value. This can be
    let _ = opt.as_mut().map(|x| x.as_mut_str());
    //~^ ERROR: called `.as_mut().map(|x| x.as_mut_str())` on an Option value. This can be
    let _ = Some(CString::new(vec![]).unwrap()).as_ref().map(CString::as_c_str);
    //~^ ERROR: called `.as_ref().map(CString::as_c_str)` on an Option value. This can be
    let _ = Some(OsString::new()).as_ref().map(OsString::as_os_str);
    //~^ ERROR: called `.as_ref().map(OsString::as_os_str)` on an Option value. This can b
    let _ = Some(PathBuf::new()).as_ref().map(PathBuf::as_path);
    //~^ ERROR: called `.as_ref().map(PathBuf::as_path)` on an Option value. This can be d
    let _ = Some(Vec::<()>::new()).as_ref().map(Vec::as_slice);
    //~^ ERROR: called `.as_ref().map(Vec::as_slice)` on an Option value. This can be done
    let _ = Some(Vec::<()>::new()).as_mut().map(Vec::as_mut_slice);
    //~^ ERROR: called `.as_mut().map(Vec::as_mut_slice)` on an Option value. This can be

    let _ = opt.as_ref().map(|x| x.deref());
    //~^ ERROR: called `.as_ref().map(|x| x.deref())` on an Option value. This can be done
    let _ = opt.clone().as_mut().map(|x| x.deref_mut()).map(|x| x.len());
    //~^ ERROR: called `.as_mut().map(|x| x.deref_mut())` on an Option value. This can be

    let vc = vec![String::new()];
    let _ = Some(1_usize).as_ref().map(|x| vc[*x].as_str()); // should not be linted

    let _: Option<&str> = Some(&String::new()).as_ref().map(|x| x.as_str()); // should not be linted

    let _ = opt.as_ref().map(|x| &**x);
    //~^ ERROR: called `.as_ref().map(|x| &**x)` on an Option value. This can be done more
    let _ = opt.as_mut().map(|x| &mut **x);
    //~^ ERROR: called `.as_mut().map(|x| &mut **x)` on an Option value. This can be done

    // Issue #5927
    let _ = opt.as_ref().map(std::ops::Deref::deref);
    //~^ ERROR: called `.as_ref().map(std::ops::Deref::deref)` on an Option value. This ca
}

#[clippy::msrv = "1.39"]
fn msrv_1_39() {
    let opt = Some(String::from("123"));
    let _ = opt.as_ref().map(String::as_str);
}

#[clippy::msrv = "1.40"]
fn msrv_1_40() {
    let opt = Some(String::from("123"));
    let _ = opt.as_ref().map(String::as_str);
    //~^ ERROR: called `.as_ref().map(String::as_str)` on an Option value. This can be don
}
