// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![warn(clippy::usage_of_ty_tykind)]
#![feature(rustc_private)]

extern crate rustc;

use rustc::ty::{self, Ty, TyKind};

fn main() {
    let sty = TyKind::Bool;

    match sty {
        TyKind::Bool => (),
        TyKind::Char => (),
        TyKind::Int(..) => (),
        TyKind::Uint(..) => (),
        TyKind::Float(..) => (),
        TyKind::Adt(..) => (),
        TyKind::Foreign(..) => (),
        TyKind::Str => (),
        TyKind::Array(..) => (),
        TyKind::Slice(..) => (),
        TyKind::RawPtr(..) => (),
        TyKind::Ref(..) => (),
        TyKind::FnDef(..) => (),
        TyKind::FnPtr(..) => (),
        TyKind::Dynamic(..) => (),
        TyKind::Closure(..) => (),
        TyKind::Generator(..) => (),
        TyKind::GeneratorWitness(..) => (),
        TyKind::Never => (),
        TyKind::Tuple(..) => (),
        TyKind::Projection(..) => (),
        TyKind::UnnormalizedProjection(..) => (),
        TyKind::Opaque(..) => (),
        TyKind::Param(..) => (),
        TyKind::Bound(..) => (),
        TyKind::Placeholder(..) => (),
        TyKind::Infer(..) => (),
        TyKind::Error => (),
    }

    if let ty::Int(int_ty) = sty {}
    if let TyKind::Int(int_ty) = sty {}

    fn ty_kind(ty_bad: TyKind<'_>, ty_good: Ty<'_>) {}
}
