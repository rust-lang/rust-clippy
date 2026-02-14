#![allow(clippy::unused_unit, clippy::needless_return)]
#![warn(clippy::getter_prefixes)]
//@no-rustfix

pub trait MyTrait {
    fn get_trait_value(&self) -> &str;
}

pub struct MyStruct {
    a: String,
    b: String,
    c: i32,
    d: Vec<usize>,
}

impl MyStruct {
    pub fn get_return_stmt(&self) -> &str {
        //~^ getter_prefixes
        return &self.a;
    }

    pub fn get_lit(self) -> Self {
        //~^ getter_prefixes
        self
    }

    pub fn get_array(&self) -> [&str; 2] {
        //~^ getter_prefixes
        [&self.a, &self.b]
    }

    pub fn get_tuple(&self) -> (&str, &i32) {
        //~^ getter_prefixes
        (&self.a, &self.c)
    }

    pub fn get_cast_value(&self) -> i64 {
        //~^ getter_prefixes
        self.c as i64
    }

    pub fn get_parens_value(&self) -> &i32 {
        //~^ getter_prefixes
        (&self.c)
    }

    pub fn get_unary(&self) -> i32 {
        //~^ getter_prefixes
        -self.c
    }

    pub fn get_binary(&self) -> i32 {
        //~^ getter_prefixes
        self.c / 2
    }

    pub fn get_method_call(&self) -> Option<&usize> {
        //~^ getter_prefixes
        self.d.first()
    }

    pub fn get_index_value(&self) -> usize {
        //~^ getter_prefixes
        self.d[0]
    }

    pub fn get_if_value(&self) -> &str {
        if self.a < self.b { &self.a } else { &self.b }
    }

    pub fn get_arm_value(&self) -> &str {
        match self.d.first() {
            Some(x) if x / 2 == 0 => &self.a,
            Some(x) => &self.b,
            None => "default",
        }
    }

    fn get_private_value(&self) -> &str {
        &self.a
    }

    pub fn get_unit(&self) {}

    pub fn get_unit_explicit(&self) -> () {}

    pub fn get_constant_value(&self) -> u32 {
        42
    }

    pub fn method_call(&self) -> Option<&usize> {
        self.d.first()
    }

    pub fn get(&self) -> &str {
        &self.a
    }

    pub fn get_mut(&mut self) -> &mut str {
        &mut self.a
    }

    pub fn get_unchecked(&self) -> usize {
        self.d[0]
    }

    pub fn get_unchecked_mut(&mut self) -> &mut usize {
        &mut self.d[0]
    }

    pub fn get_ref(&self) -> &str {
        &self.a
    }
}

impl MyTrait for MyStruct {
    fn get_trait_value(&self) -> &str {
        &self.a
    }
}

pub fn get_value() -> usize {
    42
}

fn main() {
    let mut s = MyStruct {
        a: "a".to_string(),
        b: "b".to_string(),
        c: 1,
        d: vec![1, 2, 3],
    };

    s.get_array();
    s.get_tuple();
    s.get();
    s.get_mut();
    s.get_unchecked();
    s.get_unchecked_mut();
    s.get_ref();
}
