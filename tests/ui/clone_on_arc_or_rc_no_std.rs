#![no_std]
#![warn(clippy::clone_on_arc_or_rc)]

extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

fn no_std() {
    let arc: Arc<String> = Arc::new("foo".into());
    let _: String = (*arc).clone();
    //~^ error: conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone

    let rc: Rc<String> = Rc::new("foo".into());
    let _: String = (*rc).clone();
    //~^ error: conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone

    let arc: Arc<String> = Arc::new("foo bar".into());
    let _: Vec<_> = (*arc).clone().split(" ").collect();
    //~^ error: conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone

    let rc: Rc<Vec<u32>> = Rc::new(vec![1, 2, 3]);
    let _: Vec<_> = (*rc).clone().iter().map(|x| x + 1).collect();
    //~^ error: conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone

    let _: String = (*Arc::<String>::new("foo".into())).clone();
    //~^ error: conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone
}
