#![allow(clippy::unused_unit)]
#![warn(clippy::getter_prefixes)]
//@no-rustfix

pub trait MyTrait {
    fn get_id(&self) -> usize;
    fn get_unit(&self);
    fn get_unit_explicit(&self) -> ();
    fn get_static_id() -> usize;
    fn get(&self) -> usize;
    fn get_mut(&mut self) -> usize;
    fn get_unchecked(&self) -> usize;
    fn get_unchecked_mut(&mut self) -> usize;
    fn get_ref(&self) -> usize;
}

pub struct MyTraitImpl {
    id: usize,
}

impl MyTrait for MyTraitImpl {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_unit(&self) {}

    fn get_unit_explicit(&self) -> () {}

    fn get_static_id() -> usize {
        42
    }

    fn get(&self) -> usize {
        self.id
    }

    fn get_mut(&mut self) -> usize {
        self.id
    }

    fn get_unchecked(&self) -> usize {
        self.id
    }

    fn get_unchecked_mut(&mut self) -> usize {
        self.id
    }

    fn get_ref(&self) -> usize {
        self.id
    }
}

pub struct MyStruct {
    id: usize,
}

impl MyStruct {
    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_unit(&self) {}

    pub fn get_static_id() -> usize {
        42
    }

    pub fn get(&self) -> usize {
        self.id
    }

    pub fn get_mut(&mut self) -> usize {
        self.id
    }

    pub fn get_unchecked(&self) -> usize {
        self.id
    }

    pub fn get_unchecked_mut(&mut self) -> usize {
        self.id
    }

    pub fn get_ref(&self) -> usize {
        self.id
    }
}

pub fn get_id() -> usize {
    42
}

fn main() {
    let mut s = MyStruct { id: 42 };
    s.get_id();
    MyStruct::get_static_id();
    s.get();
    s.get_mut();
    s.get_unchecked();
    s.get_unchecked_mut();
    s.get_ref();
}
