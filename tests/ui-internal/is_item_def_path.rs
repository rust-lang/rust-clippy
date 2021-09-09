// run-rustfix
// aux-build:paths.rs
#![deny(clippy::internal)]
#![feature(rustc_private)]

extern crate clippy_utils;
extern crate paths;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::is_item;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

#[allow(unused)]
use rustc_hir::LangItem;
#[allow(unused)]
use rustc_span::sym;

#[allow(unused)]
static OPTION: [&str; 3] = ["core", "option", "Option"];
#[allow(unused)]
const RESULT: &[&str] = &["core", "result", "Result"];

fn _f<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) {
    let _ = is_item(cx, ty, &OPTION);
    let _ = is_item(cx, ty, RESULT);
    let _ = is_item(cx, ty, &["core", "result", "Result"]);

    #[allow(unused)]
    let rc_path = &["alloc", "rc", "Rc"];
    let _ = clippy_utils::is_item(cx, ty, rc_path);

    let _ = is_item(cx, ty, &paths::OPTION);
    let _ = is_item(cx, ty, paths::RESULT);

    let _ = is_item(cx, ty, &["alloc", "boxed", "Box"]);
    let _ = is_item(cx, ty, &["core", "mem", "maybe_uninit", "MaybeUninit", "uninit"]);
}

fn main() {}
