#![warn(clippy::use_destructuring)]
#![allow(unused, clippy::no_effect, clippy::needless_pass_by_value)]

use std::fmt;

// ============ SHOULD LINT (3+ fields, default threshold) ============

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

// --- Function parameter (not self, so not linted with default scope="self") ---
fn sum_vec3(v: Vec3) -> f32 {
    v.x + v.y + v.z
}

// --- Local variable (not self, so not linted with default scope="self") ---
fn use_local() -> f32 {
    let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    v.x + v.y + v.z
}

// Tuple struct with 3 fields
struct TupleStruct3(i32, i32, i32);

impl TupleStruct3 {
    fn sum(self) -> i32 {
        self.0 + self.1 + self.2
        //~^ use_destructuring
    }
}

// ============ SHOULD NOT LINT ============

// --- Only 2 fields (below default threshold of 3) ---
struct Pair {
    a: i32,
    b: i32,
}

fn add_pair(p: Pair) -> i32 {
    p.a + p.b
}

struct TupleStruct2(i32, i32);

impl TupleStruct2 {
    fn sum(self) -> i32 {
        self.0 + self.1
    }
}

// --- Only one field ---
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

// --- Non-exhaustive struct ---
#[non_exhaustive]
struct NonExhaustive {
    x: i32,
    y: i32,
    z: i32,
}

impl NonExhaustive {
    fn sum(self) -> i32 {
        self.x + self.y + self.z
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

// --- Field name would collide with an existing local binding ---
fn name_collision(v: Vec3) -> f32 {
    let x = 42.0_f32;
    x + v.x + v.y + v.z
}
