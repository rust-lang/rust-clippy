// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub struct MyStruct {
    id: usize
}

impl MyStruct {
    pub fn get_id(&self) -> usize {
        self.id
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

fn main() {
   let mut s = MyStruct { id: 42 };
   s.get_id();
   s.get();
   s.get_mut();
   s.get_unchecked();
   s.get_unchecked_mut();
   s.get_ref();
}
