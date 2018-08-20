// #![cfg_attr(feature = "clippy", deny(dropping_temporary_with_side_effect))]
#![allow(mutex_atomic)]
#![feature(tool_attributes)]
#![feature(stmt_expr_attributes)]

#![deny(dropping_temporary_with_side_effect)]

#[allow(unused_imports)]
#[allow(unused_variables)]
use std::sync::{Mutex, MutexGuard};

use std::ops::Index;
use std::sync::RwLock;
use std::cell::RefCell;

trait MyTrait {
	type AssociatedType;
	fn get(self) -> Self::AssociatedType;
}

#[derive(Clone, Copy, Debug)]
struct S {
	x: u8
}

struct MutexContainer<'a> {
	rwlmg: MutexGuard<'a, S>
}

impl<'a> MyTrait for MutexContainer<'a> {
	type AssociatedType = MutexGuard<'a, S>;
	fn get(self) -> MutexGuard<'a, S> {
		self.rwlmg // Should not trigger
	}
}

impl<'a> MutexContainer<'a> {
	#[allow(dead_code)]
	fn get_wrapped(self) -> MutexGuard<'a, S> {
		self.get() // Should not trigger
	}
}

#[allow(needless_lifetimes)]
#[allow(dead_code)]
fn get<'a>(m: MutexContainer<'a>) -> MutexGuard<'a, S> {
	m.get() // Should not trigger
}

#[allow(dead_code)]
fn g<T: MyTrait> (param: T) {
	let _ = param.get(); // Should not trigger.
}

#[allow(dead_code)]
fn f<'a, T: MyTrait<AssociatedType = MutexGuard<'a, S>>> (param: T) {
	let _ = param.get();  // Should trigger.
}

struct MutexIndexable {
}

impl<'a, T> Index<&'a MutexGuard<'a, T>> for MutexIndexable {
	type Output = u8;
	fn index(&self, _mg: &'a MutexGuard<T>) -> &u8 { &5 }
}

fn should_trigger() {
	let mutex = Mutex::new(S{x: 4});
	
	let _   = mutex.lock();     // Should trigger.
	let _   = (mutex.lock(), ); // Should trigger.
	let _   = [mutex.lock()];   // Should trigger.
	let _   = {mutex.lock()};   // Should trigger.
	let _a1 = mutex.lock().unwrap().x; // Should trigger.
	let _a2 = MutexContainer{rwlmg: mutex.lock().unwrap()}; // Should trigger
	let _a3 = [1,2,3,4,5][mutex.lock().unwrap().x as usize]; // Should trigger
	let _a4 = MutexIndexable{}[&mutex.lock().unwrap()]; // Should trigger
	let _a5 = *mutex.lock().unwrap(); // Should trigger
	let _a6 = [mutex.lock(), mutex.lock()].len(); // Should trigger
	println!("{:?}", (mutex.lock(), 4)); // Should trigger
	let _a7 = (&mutex.lock()).is_err(); // Should trigger
	
	let mutex_bool = Mutex::new(true);
	if *mutex_bool.lock().unwrap() { // Should trigger
		println!("Do something.");
	}

	match mutex_bool.lock() { // should trigger
		Ok(ref x) if **x => {
			println!("Do something.");
		}
		_ => ()
	}
	
	let rwlock = RwLock::new(0);
	let _ = rwlock.read();  // Should trigger
	let _ = rwlock.write(); // Should trigger
	
	let refcell = RefCell::new(0);
	let _ = refcell.borrow();     // Should trigger
	let _ = refcell.borrow_mut(); // Should trigger
	
	
	let cache: RwLock<Option<u8>> = RwLock::new(None);
	let _value = {
		match cache.read() { // Should trigger
			Ok(ref y) if y.is_some() => Some(y.unwrap()),
			_ => None,
		}
	}
	.unwrap_or_else(|| {
		let mut _data = cache.write().unwrap();
		*_data = Some(0);
		0
    });
}

fn should_not_trigger() {
	let mutex = Mutex::new(S{x: 4});
	let _a1 = mutex.lock().unwrap(); // Should not trigger
	
	let mutex_bool = Mutex::new(true);
	let must_do_something : bool;
	{
		let value = mutex_bool.lock().unwrap(); // Should not trigger
		must_do_something = *value;
	}
	if must_do_something {
		println!("Do something.");
	}
	
	{
		let value = mutex_bool.lock();
		match value {
			Ok(ref x) if **x => {
					println!("Do something");
			},
			_ => ()
		}
	}
	
	{
		let _data = mutex.lock().unwrap();
		println!("Do something with data.");
	}
}

fn main() {
	should_trigger();
	should_not_trigger();
}
