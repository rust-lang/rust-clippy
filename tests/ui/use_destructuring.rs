#![warn(clippy::use_destructuring)]
#![allow(unused, clippy::no_effect, clippy::needless_pass_by_value)]

use std::fmt;

// ============ SHOULD LINT ============

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn sum(self) -> f32 {
        self.x + self.y + self.z
        //~^ use_destructuring
    }

    fn sum_ref(&self) -> f32 {
        self.x + self.y + self.z
        //~^ use_destructuring
    }
}

// Field access passed as arguments to a macro
impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
        //~^ use_destructuring
    }
}

struct Pair {
    a: i32,
    b: i32,
}

impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.a, self.b)
        //~^ use_destructuring
    }
}

// All fields accessed on a function parameter
fn add_pair(p: Pair) -> i32 {
    p.a + p.b
    //~^ use_destructuring
}

// All fields accessed on a local variable
fn use_local() -> i32 {
    let p = Pair { a: 1, b: 2 };
    p.a + p.b
    //~^ use_destructuring
}

// Tuple struct
struct TupleStruct(i32, i32);

impl TupleStruct {
    fn sum(self) -> i32 {
        self.0 + self.1
        //~^ use_destructuring
    }
}

// ============ SHOULD NOT LINT ============

// --- Only one field (fewer than 2) ---
struct Single {
    x: f32,
}

impl Single {
    fn get(self) -> f32 {
        self.x
    }
}

// --- Not all fields accessed ---
struct Triple {
    a: i32,
    b: i32,
    c: i32,
}

impl Triple {
    fn partial(self) -> i32 {
        self.a + self.b
    }
}

// --- Variable used in non-field-access context ---
fn take_vec3(_v: Vec3) {}

fn pass_whole(v: Vec3) -> f32 {
    let _ = v.x + v.y + v.z;
    take_vec3(v);
    0.0
}

// --- Field is mutated ---
impl Pair {
    fn mutate(&mut self) {
        self.a = 1;
        self.b = 2;
    }
}

// --- Tuple struct (should still lint) ---

// --- Non-exhaustive struct ---
#[non_exhaustive]
struct NonExhaustive {
    x: i32,
    y: i32,
}

impl NonExhaustive {
    fn sum(self) -> i32 {
        self.x + self.y
    }
}

// --- Union ---
union MyUnion {
    a: i32,
    b: f32,
}

// --- From macro expansion ---
macro_rules! access_fields {
    ($v:expr) => {
        $v.x + $v.y + $v.z
    };
}

fn use_macro(v: Vec3) -> f32 {
    access_fields!(v)
}

// --- Variable also returned/moved ---
fn return_whole(v: Vec3) -> Vec3 {
    let _ = v.x + v.y + v.z;
    v
}

// --- Only some fields accessed multiple times but not all ---
impl Triple {
    fn double_a(self) -> i32 {
        self.a + self.a
    }
}
