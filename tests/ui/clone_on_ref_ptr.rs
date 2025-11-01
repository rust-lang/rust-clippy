#![warn(clippy::clone_on_ref_ptr)]

use std::rc::{Rc, Weak as RcWeak};
use std::sync::{Arc, Weak as ArcWeak};

fn main() {}

fn clone_on_ref_ptr(rc: Rc<str>, rc_weak: RcWeak<str>, arc: Arc<str>, arc_weak: ArcWeak<str>) {
    rc.clone();
    //~^ clone_on_ref_ptr
    rc_weak.clone();
    //~^ clone_on_ref_ptr
    arc.clone();
    //~^ clone_on_ref_ptr
    arc_weak.clone();
    //~^ clone_on_ref_ptr

    Rc::clone(&rc);
    Arc::clone(&arc);
    RcWeak::clone(&rc_weak);
    ArcWeak::clone(&arc_weak);
}

trait SomeTrait {}
struct SomeImpl;
impl SomeTrait for SomeImpl {}

fn trait_object() {
    let x = Arc::new(SomeImpl);
    let _: Arc<dyn SomeTrait> = x.clone();
    //~^ clone_on_ref_ptr
}

mod issue2076 {
    use std::rc::Rc;

    macro_rules! try_opt {
        ($expr: expr) => {
            match $expr {
                Some(value) => value,
                None => return None,
            }
        };
    }

    fn func() -> Option<Rc<u8>> {
        let rc = Rc::new(42);
        Some(try_opt!(Some(rc)).clone())
        //~^ clone_on_ref_ptr
    }
}

mod issue15009 {
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn main() {
        let counter = AtomicU32::new(0);
        let counter_ref = &counter;
        let factorial = Rc::new_cyclic(move |rec| {
            let rec = rec.clone() as Weak<dyn Fn(u32) -> u32>;
            //~^ clone_on_ref_ptr
            move |x| {
                // can capture env
                counter_ref.fetch_add(1, Ordering::Relaxed);
                match x {
                    0 => 1,
                    x => x * rec.upgrade().unwrap()(x - 1),
                }
            }
        });
        println!("{}", factorial(5)); // 120
        println!("{}", counter.load(Ordering::Relaxed)); // 6
        println!("{}", factorial(7)); // 5040
        println!("{}", counter.load(Ordering::Relaxed)); // 14
    }
}

fn issue15741(mut rc: Rc<str>, ref_rc: &Rc<str>, refmut_rc: &mut Rc<str>) {
    rc.clone();
    //~^ clone_on_ref_ptr
    ref_rc.clone();
    //~^ clone_on_ref_ptr
    refmut_rc.clone();
    //~^ clone_on_ref_ptr

    // The following cases already cause warn-by-default lints to fire, and the suggestion just makes
    // another set of warn-by-default lints to fire, so this is probably fine

    #[allow(clippy::needless_borrow, clippy::unnecessary_mut_passed)] // before the suggestion
    #[allow(clippy::double_parens)] // after the suggestion
    {
        (rc).clone();
        //~^ clone_on_ref_ptr
        (&rc).clone();
        //~^ clone_on_ref_ptr
        (&mut rc).clone();
        //~^ clone_on_ref_ptr
    };
}
