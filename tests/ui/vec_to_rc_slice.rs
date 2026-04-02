#![warn(clippy::vec_to_rc_slice)]

use std::rc::Rc;
use std::sync::Arc;

fn main() {
    // Should lint: Vec<T>.into() -> Arc<[T]>
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = v.into();
    //~^ vec_to_rc_slice

    // Should lint: Arc::from(vec)
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = Arc::from(v);
    //~^ vec_to_rc_slice

    // Should lint: From::from(vec) when target is Arc<[T]>
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = From::from(v);
    //~^ vec_to_rc_slice

    // Should lint: <Arc<[u8]> as From<Vec<u8>>>::from(vec)
    let v: Vec<u8> = vec![1, 2, 3];
    let _a = <Arc<[u8]> as From<Vec<u8>>>::from(v);
    //~^ vec_to_rc_slice

    // Should lint: Vec<T>.into() -> Rc<[T]>
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = v.into();
    //~^ vec_to_rc_slice

    // Should lint: Rc::from(vec)
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = Rc::from(v);
    //~^ vec_to_rc_slice

    // Should lint: From::from(vec) when target is Rc<[T]>
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = From::from(v);
    //~^ vec_to_rc_slice

    // Should lint: <Rc<[u8]> as From<Vec<u8>>>::from(vec)
    let v: Vec<u8> = vec![1, 2, 3];
    let _a = <Rc<[u8]> as From<Vec<u8>>>::from(v);
    //~^ vec_to_rc_slice

    // Should NOT lint: Vec<T>.into() -> something else
    let v: Vec<u8> = vec![1, 2, 3];
    let _b: Box<[u8]> = v.into();

    // Should NOT lint: non-Vec into Arc<[T]>
    let _c: Arc<[u8]> = Arc::from([1u8, 2, 3].as_slice());

    // Should NOT lint: Vec into Arc (not Arc<[T]>)
    let v: Vec<u8> = vec![1, 2, 3];
    let _d: Arc<Vec<u8>> = Arc::new(v);

    // Should NOT lint: the recommended pattern
    let v: Vec<u8> = vec![1, 2, 3];
    let _e: Arc<Box<[u8]>> = Arc::new(v.into_boxed_slice());

    // Should NOT lint: Rc recommended pattern
    let v: Vec<u8> = vec![1, 2, 3];
    let _f: Rc<Box<[u8]>> = Rc::new(v.into_boxed_slice());
}
