#[macro_export]
macro_rules! expr {
    () => {
        1
    };
}

#[macro_export]
macro_rules! stmt {
    () => {
        let _x = ();
    };
}

#[macro_export]
macro_rules! ty {
    () => { &'static str };
}

#[macro_export]
macro_rules! pat {
    () => {
        _
    };
}

#[macro_export]
macro_rules! item {
    () => {
        const ITEM: usize = 1;
    };
}

#[macro_export]
macro_rules! binop {
    ($t:tt) => {
        $t + $t
    };
}

#[macro_export]
macro_rules! attr {
    ($i:item) => {
        #[repr(C)]
        $i
    };
}

#[macro_export]
macro_rules! attr_item {
    () => {
        #[repr(C)]
        struct AttrItem {}
    };
}

#[macro_export]
macro_rules! attr_expr {
    ($e:expr) => {{
        #[repr(C)]
        struct AttrExpr {}
        $e
    }};
}

#[macro_export]
macro_rules! wrap_attr_expr {
    ($e:expr) => {{
        #[allow(clippy::disallowed_macros)]
        let x = $crate::attr_expr!($e);
        x
    }};
}
