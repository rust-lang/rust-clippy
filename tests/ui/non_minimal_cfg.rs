//@compile-flags: --cfg foo --cfg bar

#[cfg(all(foo))]
//~^ non_minimal_cfg
fn all() {}

#[cfg(any(foo))]
//~^ non_minimal_cfg
fn any() {}

#[cfg(any(any(foo), all(not(bar))))]
//~^ non_minimal_cfg
//~| non_minimal_cfg
fn mixed() {}

#[cfg(all())]
//~^ non_minimal_cfg
fn empty_all() {}

#[cfg(any(true, any()))]
//~^ non_minimal_cfg
fn empty_any() {}

#[cfg_attr(all(foo), doc = "all")]
//~^ non_minimal_cfg
#[cfg_attr(any(any(foo), all(not(bar))), doc = "mixed")]
//~^ non_minimal_cfg
//~| non_minimal_cfg
#[cfg_attr(any(not_active), doc = "catches not active cfg_attrs")]
//~^ non_minimal_cfg
fn cfg_attr() {}

macro_rules! in_macro {
    ($($tt:tt)*) => {
        #[cfg(any($($tt)*))]
        struct FromMacro;
    }
}

in_macro!(foo);

#[clippy::msrv = "1.87"]
#[cfg_attr(any(), inline)]
fn msrv_1_87() {}

#[clippy::msrv = "1.88"]
#[cfg_attr(any(), inline)]
//~^ non_minimal_cfg
fn msrv_1_88() {}
