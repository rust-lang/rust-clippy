#![warn(clippy::duplicate_fn_bodies)]
#![allow(dead_code)]

fn exact_free_a(x: i32) -> i32 {
    let y = x + 1;
    y * 2
}

#[rustfmt::skip]
fn exact_free_b(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 1;
    y * 2
}

fn comments_and_whitespace_a(x: i32) -> i32 {
    // The comment is not part of the canonical token stream.
    let y = x + 2;
    y * 3
}

#[rustfmt::skip]
fn comments_and_whitespace_b(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 2; // Neither is this comment.

    y * 3
}

fn class_a(x: i32) -> i32 {
    let y = x + 3;
    y * 4
}

#[rustfmt::skip]
fn class_b(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 3;
    y * 4
}

#[rustfmt::skip]
fn class_c(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 3;
    y * 4
}

mod nested {
    pub fn cross_module_a(x: i32) -> i32 {
        let y = x + 4;
        y * 5
    }
}

mod another_nested {
    #[rustfmt::skip]
    pub fn cross_module_b(x: i32) -> i32 { //~ duplicate_fn_bodies
        let y = x + 4;
        y * 5
    }
}

struct Pair {
    value: i32,
}

impl Pair {
    fn inherent_method_a(&self) -> i32 {
        let y = self.value + 5;
        y * 6
    }

    #[rustfmt::skip]
    fn inherent_method_b(&self) -> i32 { //~ duplicate_fn_bodies
        let y = self.value + 5;
        y * 6
    }
}

fn consume_i32(_: i32) {}
fn consume_str(_: &str) {}

fn numeric_spelling_a() {
    let value = 1000;
    consume_i32(value);
}

fn numeric_spelling_b() {
    let value = 1_000;
    consume_i32(value);
}

fn string_spelling_a() {
    let value = "hello";
    consume_str(value);
}

fn string_spelling_b() {
    let value = r#"hello"#;
    consume_str(value);
}

fn raw_identifier_a(value: i32) -> i32 {
    let r#type = value + 1;
    r#type * 2
}

#[rustfmt::skip]
fn raw_identifier_b(value: i32) -> i32 { //~ duplicate_fn_bodies
    let r#type = value + 1;
    r#type * 2
}

macro_rules! source_level_macro {
    ($value:expr) => {
        $value + 1
    };
}

fn macro_body_a(x: i32) -> i32 {
    let y = source_level_macro!(x);
    y * 2
}

#[rustfmt::skip]
fn macro_body_b(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = source_level_macro!(x);
    y * 2
}

fn renamed_local_a(x: i32) -> i32 {
    let local = x + 6;
    local * 7
}

fn renamed_local_b(x: i32) -> i32 {
    let other = x + 6;
    other * 7
}

fn renamed_reference_a(input: i32) -> i32 {
    let y = input + 8;
    y * 9
}

fn renamed_reference_b(value: i32) -> i32 {
    let y = value + 8;
    y * 9
}

fn different_literal_a(x: i32) -> i32 {
    let y = x + 10;
    y * 11
}

fn different_literal_b(x: i32) -> i32 {
    let y = x + 12;
    y * 11
}

fn different_operator_a(x: i32) -> i32 {
    let y = x + 13;
    y * 14
}

fn different_operator_b(x: i32) -> i32 {
    let y = x - 13;
    y * 14
}

fn different_order_a(x: i32) -> i32 {
    let y = x + 15;
    y * 16
}

fn different_order_b(x: i32) -> i32 {
    let y = x * 16;
    y + 15
}

fn slightly_changed_a(x: i32) -> i32 {
    let y = x + 17;
    y * 18
}

fn slightly_changed_b(x: i32) -> i32 {
    let y = x + 17;
    y * 19
}

fn tiny_a() -> bool {
    true
}

fn tiny_b() -> bool {
    true
}

macro_rules! generated_functions {
    ($name:ident) => {
        fn $name(x: i32) -> i32 {
            let y = x + 1;
            y * 2
        }
    };
}

generated_functions!(generated_a);
generated_functions!(generated_b);

#[derive(Clone, Copy, Debug)]
struct Derived {
    value: i32,
}

trait TraitBoilerplate {
    fn trait_default_a(&self, x: i32) -> i32 {
        let y = x + 23;
        y * 24
    }

    #[rustfmt::skip]
    fn trait_default_b(&self, x: i32) -> i32 { //~ duplicate_fn_bodies
        let y = x + 23;
        y * 24
    }
}

struct TraitImplementation;

impl TraitBoilerplate for TraitImplementation {
    fn trait_default_a(&self, x: i32) -> i32 {
        let y = x + 25;
        y * 26
    }

    #[rustfmt::skip]
    fn trait_default_b(&self, x: i32) -> i32 { //~ duplicate_fn_bodies
        let y = x + 25;
        y * 26
    }
}

fn allowed_source(x: i32) -> i32 {
    let y = x + 20;
    y * 21
}

#[allow(clippy::duplicate_fn_bodies)]
fn allowed_duplicate(x: i32) -> i32 {
    let y = x + 20;
    y * 21
}

#[allow(clippy::duplicate_fn_bodies)]
fn allowed_canonical(x: i32) -> i32 {
    let y = x + 22;
    y * 23
}

#[rustfmt::skip]
fn enabled_duplicate(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 22;
    y * 23
}

fn expected_canonical(x: i32) -> i32 {
    let y = x + 27;
    y * 28
}

#[expect(clippy::duplicate_fn_bodies)]
fn expected_duplicate(x: i32) -> i32 {
    let y = x + 27;
    y * 28
}

#[expect(clippy::duplicate_fn_bodies)]
fn expected_primary(x: i32) -> i32 {
    let y = x + 29;
    y * 30
}

#[rustfmt::skip]
fn expected_primary_duplicate(x: i32) -> i32 { //~ duplicate_fn_bodies
    let y = x + 29;
    y * 30
}

fn main() {}
