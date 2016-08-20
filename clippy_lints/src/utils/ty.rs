use rustc::ty::Ty;

fn is_internally_mutable(ty: Ty) {
    match ty.sty {
        TyStruct(ref adt, ref subst) => {false}
        TyEnum(ref adt, ref subst) => {false}
        TyBox(t) | TySlice(t) | TyArray(t, _) => is_internally_mutable(t),
        TyTuple(ref tys) => tys.iter().any(is_internally_mutable),
        TyRawPtr(ref tm) | TyRef(_, ref tm) => tm.mutbl == Mutability::MutMutable || is_internally_mutable(tm.ty),
        TyProjection(ref pty) => {
            //TODO
            false
        }
        _ => false
    }
}
