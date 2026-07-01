#![warn(clippy::trusted_asref)]

unsafe fn use_slice<T>(_s: *const T, _len: usize) {}

unsafe fn use_ptr<T>(_s: *const T) {}

fn bad_generic<T: AsRef<str>>(s: T) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
        //~^ trusted_asref
    }
}

fn bad_impl(s: impl AsRef<[u8]>) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
        //~^ trusted_asref
    }
}

fn bad_where<T>(s: T)
where
    T: AsRef<[u8]>,
{
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
        //~^ trusted_asref
    }
}

fn bad_nested_unsafe<T: AsRef<str>>(s: T) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
        //~^ trusted_asref
        unsafe {
            use_slice(s.as_ref().as_ptr(), s.as_ref().len());
            //~^ trusted_asref
        }
    }
}

fn bad_multiple_expr<T: AsRef<[u8]>>(s: T) {
    unsafe {
        let p = s.as_ref().as_ptr();
        //~^ trusted_asref
        let len = s.as_ref().len();
        use_slice(p, len);
    }
}

fn bad_multiple_calls<T: AsRef<[u8]>>(s: T) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
        //~^ trusted_asref
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
    }
}

fn good_generic<T: AsRef<str>>(s: T) {
    let s = s.as_ref();
    unsafe {
        use_slice(s.as_ptr(), s.len());
    }
}

fn good_impl(s: impl AsRef<[u8]>) {
    let s = s.as_ref();
    unsafe {
        use_slice(s.as_ptr(), s.len());
    }
}

fn good_where<T>(s: T)
where
    T: AsRef<[u8]>,
{
    let s = s.as_ref();
    unsafe {
        use_slice(s.as_ptr(), s.len());
    }
}

fn good_trustworthy_std(s: std::rc::Rc<str>) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
    }
}

struct TrustworthyStr(String);

impl AsRef<str> for TrustworthyStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn good_trustworthy_crate_local(s: TrustworthyStr) {
    unsafe {
        use_slice(s.as_ref().as_ptr(), s.as_ref().len());
    }
}

fn good_unrelated_calls(a: impl AsRef<u32>, b: impl AsRef<u32>) {
    unsafe {
        use_ptr(a.as_ref() as *const u32);
        use_ptr(b.as_ref() as *const u32);
    }
}

fn good_no_unsafe(s: impl AsRef<str>) {
    let _ptr = s.as_ref().as_ptr();
    let _len = s.as_ref().len();
}
