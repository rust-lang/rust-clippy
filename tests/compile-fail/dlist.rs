#![feature(plugin, collections)]
#![feature(associated_type_defaults)]
#![feature(associated_consts)]

#![plugin(clippy)]
#![deny(clippy)]
#![allow(dead_code)]

extern crate collections;
use collections::linked_list::LinkedList;

trait Foo {
    type Baz = LinkedList<u8>; //~ ERROR I see you're using a LinkedList!
    fn foo(LinkedList<u8>); //~ ERROR I see you're using a LinkedList!
    const BAR : Option<LinkedList<u8>>; //~ ERROR I see you're using a LinkedList!
}

// ok, we don’t want to warn for implementations, see #605
impl Foo for LinkedList<u8> {
    fn foo(_: LinkedList<u8>) {}
    const BAR : Option<LinkedList<u8>> = None;
}

struct Bar;
impl Bar {
    fn foo(_: LinkedList<u8>) {} //~ ERROR I see you're using a LinkedList!
}

pub fn test(foo: LinkedList<u8>) { //~ ERROR I see you're using a LinkedList!
    println!("{:?}", foo)
}

pub fn test_ret() -> Option<LinkedList<u8>> { //~ ERROR I see you're using a LinkedList!
    unimplemented!();
}

fn main(){
    test(LinkedList::new());
}
