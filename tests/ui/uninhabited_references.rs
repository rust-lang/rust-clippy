#![warn(clippy::uninhabited_references)]
#![feature(never_type)]

//~v uninhabited_references
fn ret_uninh_ref() -> &'static std::convert::Infallible {
    unsafe { std::mem::transmute(&()) }
}

macro_rules! ret_something {
    ($name:ident, $ty:ty) => {
        //~v uninhabited_references
        fn $name(x: &$ty) -> &$ty {
            &*x //~ uninhabited_references
        }
    };
}

ret_something!(id_u32, u32);
ret_something!(id_never, !);

fn main() {
    let x = ret_uninh_ref();
    let _ = *x; //~ uninhabited_references
}
