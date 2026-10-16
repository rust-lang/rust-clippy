#![warn(clippy::deref_addrof)]
#![allow(clippy::borrow_deref_ref)]
#![deny(static_mut_refs)]

use std::ops::AddAssign;

pub static mut N: i32 = 1000;

fn main() {}

fn reproducer() {
    let n: &mut i32 = unsafe { &mut *&raw mut N };
    *n += 1;
    println!("{}", unsafe { *&raw const N });
    //~^ deref_addrof
    unsafe {
        println!("{}", *&raw const N);
    }
}

fn static_mut_explicit_reborrows() {
    unsafe {
        let _ = &*&raw const N;
        let _ = &*&raw mut N;
        let _ = &mut *&raw mut N;

        let _ = &*(&raw const N);
        let _ = &mut *(&raw mut N);
        let _ = &(*(&raw const N));
        let _ = &mut (*(&raw mut N));

        let _ = &*{ &raw const N };
        let _ = &mut *{ &raw mut N };
    }
}

fn local_reborrows() {
    let mut n = 0;
    unsafe {
        let _ = &*&raw const n;
        //~^ deref_addrof
        let _ = &*&raw mut n;
        //~^ deref_addrof
        let _ = &mut *&raw mut n;
        //~^ deref_addrof
    }
}

fn still_lint() {
    unsafe {
        let _ = *&raw const N;
        //~^ deref_addrof
        let _ = *&raw mut N;
        //~^ deref_addrof
        *&raw mut N = 1;
        //~^ deref_addrof

        let _ = &raw const *&raw const N;
        //~^ deref_addrof
        let _ = &raw mut *&raw mut N;
        //~^ deref_addrof
    }

    let mut n = 0;
    let _ = &*&n;
    //~^ deref_addrof
    let _ = &mut *&mut n;
    //~^ deref_addrof
}

struct Holder {
    field: Vec<i32>,
}

static mut V: Vec<i32> = Vec::new();
static mut S: Option<String> = None;
static mut P: Holder = Holder { field: Vec::new() };
static mut ARR: [i32; 1] = [0];
static mut STR: &str = "";

#[allow(clippy::toplevel_ref_arg)]
fn static_mut_borrows() {
    unsafe {
        (*&raw mut V).push(1);
        let _ = (*&raw const S).is_some();
        let _ = (*&raw const V).len();
        (*&raw mut P).field.push(1);
        let _ = (*&raw const P).field.len();
        let _ = &mut (*&raw mut P).field;

        for _ in (*&raw const V).iter() {}
        let _ = || (*&raw const V).len();
        let _: &[i32] = &*&raw const V;
        let _ = (*&raw const ARR).len();
        #[allow(dangerous_implicit_autorefs)]
        let _ = (*&raw const STR).len();

        let ref _n = *&raw const N;
        let ref mut _n = *&raw mut N;
        let ref _field = (*&raw const P).field;

        (*&raw mut P.field).push(1);
        let _ = &*&raw const P.field;
        let ref _field = *&raw const P.field;
    }
}

#[derive(Clone, Copy)]
struct Counter(i32);

impl AddAssign<i32> for Counter {
    fn add_assign(&mut self, value: i32) {
        self.0 += value;
    }
}

static mut C: Counter = Counter(0);
static mut T: (i32, i32) = (0, 0);

#[allow(dangerous_implicit_autorefs)]
fn static_mut_non_borrowing_uses() {
    unsafe {
        (*&raw mut V)[0] = 1;
        //~^ deref_addrof
        let _ = (*&raw const V)[0];
        //~^ deref_addrof
        (*&raw mut P).field[0] = 1;
        //~^ deref_addrof

        (*&raw mut ARR)[0] = 1;
        //~^ deref_addrof
        let _ = (*&raw const ARR)[0];
        //~^ deref_addrof
        let _ = &mut (*&raw mut ARR)[0];
        //~^ deref_addrof

        let _ = (*&raw const V) == Vec::<i32>::new();
        //~^ deref_addrof
        let _ = match *&raw const S {
            //~^ deref_addrof
            Some(ref s) => s.len(),
            None => 0,
        };
        if let Some(ref s) = *&raw const S {
            //~^ deref_addrof
            let _ = s.len();
        }
        let Some(ref _s) = *&raw const S else {
            //~^ deref_addrof
            return;
        };
        let (ref _a, _) = *&raw const T;
        //~^ deref_addrof
        *&raw mut C += 1;
        //~^ deref_addrof
        let _ = (*&raw const N).abs();
        //~^ deref_addrof
        let _ = (*&raw const C).0;
        //~^ deref_addrof
        let _ = (*&raw const C.0).abs();
        //~^ deref_addrof
        let _ = &mut { *&raw mut N };
        //~^ deref_addrof
    }
}

fn fn_local_static_mut() {
    static mut SM: Vec<i32> = Vec::new();
    unsafe {
        let _ = &mut *&raw mut SM;
        (*&raw mut SM).push(1);
    }
}

mod m {
    pub static mut MN: i32 = 0;
    pub static mut MV: Vec<i32> = Vec::new();
}

struct Outer {
    inner: Holder,
}

static mut Q: Outer = Outer {
    inner: Holder { field: Vec::new() },
};
static mut TV: (i32, Vec<i32>) = (0, Vec::new());
static mut RV: &Vec<i32> = &Vec::new();

trait ByValue {
    fn by_value(self) -> usize;
}

impl ByValue for &Vec<i32> {
    fn by_value(self) -> usize {
        self.len()
    }
}

#[allow(clippy::toplevel_ref_arg, clippy::vec_init_then_push)]
fn static_mut_paths_and_projections() {
    unsafe {
        let _ = &mut *&raw mut m::MN;
        (*&raw mut m::MV).push(1);
        (*&raw mut Q).inner.field.push(1);
        let _ = &mut (*&raw mut Q).inner.field;
        let _ = &(*&raw const Q.inner).field;
        let _ = &mut (*&raw mut TV).1;
        (*&raw mut TV).1.push(1);
        let _ = (*&raw const RV).clone();
        let ref _s @ Some(_) = *&raw const S else { return };
        let ref _n: i32 = *&raw const N;
        (*&raw mut V).push((*&raw const V).len() as i32);
        let _ = &mut *std::ptr::addr_of_mut!(N);

        let _ = *&raw const m::MN;
        //~^ deref_addrof
        let _ = (*&raw const TV).0;
        //~^ deref_addrof
        (*&raw mut TV).0 += 1;
        //~^ deref_addrof
        (*&raw mut Q).inner.field = Vec::new();
        //~^ deref_addrof
        let mut v = Vec::new();
        v.push(*&raw const N);
        //~^ deref_addrof
        let _ = (*&raw const RV).by_value();
        //~^ deref_addrof
    }
}

#[allow(clippy::toplevel_ref_arg)]
fn non_static_mut_places() {
    static ST: Vec<i32> = Vec::new();
    struct P {
        v: Vec<i32>,
    }
    let mut n = 0;
    let mut v: Vec<i32> = Vec::new();
    let p = P { v: Vec::new() };
    let pp: *const P = &p;
    unsafe {
        let _ = &*&raw const n;
        //~^ deref_addrof
        let _ = &mut *&raw mut n;
        //~^ deref_addrof
        let _ = (*&raw const v).len();
        //~^ deref_addrof
        (*&raw mut v).push(1);
        //~^ deref_addrof
        let ref r = *&raw const n;
        //~^ deref_addrof
        let _ = (*&raw const p).v.len();
        //~^ deref_addrof
        let _ = &*&raw const ST;
        //~^ deref_addrof
        let _ = (*&raw const ST).len();
        //~^ deref_addrof
        let _ = (*&raw const (*pp).v).len();
        //~^ deref_addrof
        let _ = &*&raw const (*pp).v;
        //~^ deref_addrof
    }
}
