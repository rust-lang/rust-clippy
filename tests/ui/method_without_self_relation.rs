#![warn(clippy::method_without_self_relation)]
#![allow(dead_code, unused_variables)]

use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
struct Calculator {
    precision: u32,
}

impl Calculator {
    // Should trigger lint - no Self relation at all
    fn add(a: i32, b: i32) -> i32 {
        //~^ ERROR: method `add` has no relationship to `Self`
        a + b
    }

    // Should trigger lint - no Self relation
    fn multiply(x: i32, y: i32) -> i32 {
        //~^ ERROR: method `multiply` has no relationship to `Self`
        x * y
    }

    // Should NOT trigger - has &self
    fn get_precision(&self) -> u32 {
        self.precision
    }

    // Should NOT trigger - has &mut self
    fn set_precision(&mut self, precision: u32) {
        self.precision = precision;
    }

    // Should NOT trigger - has self
    fn consume(self) -> u32 {
        self.precision
    }

    // Should NOT trigger - returns Self
    fn new(precision: u32) -> Self {
        Self { precision }
    }

    // Should NOT trigger - takes Self as parameter
    fn from_self(other: Self) -> u32 {
        other.precision
    }

    // Should NOT trigger - uses Self in Option
    fn from_option(opt: Option<Self>) -> u32 {
        opt.map(|s| s.precision).unwrap_or(0)
    }

    // Should NOT trigger - returns Option<Self>
    fn maybe_new(precision: u32) -> Option<Self> {
        if precision > 0 { Some(Self { precision }) } else { None }
    }

    // Should NOT trigger - uses Result<Self, E>
    fn try_new(precision: u32) -> Result<Self, String> {
        if precision > 0 {
            Ok(Self { precision })
        } else {
            Err("Invalid precision".to_string())
        }
    }

    // Should NOT trigger - returns Result with Self in error
    fn result_with_self_error(precision: u32) -> Result<i32, Self> {
        if precision == 0 {
            Err(Self { precision: 1 })
        } else {
            Ok(precision as i32)
        }
    }

    // Should NOT trigger - Self in Vec
    fn many(count: usize, precision: u32) -> Vec<Self> {
        vec![Self { precision }; count]
    }

    // Should NOT trigger - &Self parameter
    fn compare(a: &Self, b: &Self) -> bool {
        a.precision == b.precision
    }

    // Should NOT trigger - exotic self receiver
    fn from_arc(self: Arc<Self>) -> u32 {
        self.precision
    }

    // Should NOT trigger - Pin<&mut Self>
    fn pinned_method(self: Pin<&mut Self>) {
        // Do something
    }

    // Should NOT trigger - Self in tuple
    fn tuple_with_self(x: i32) -> (i32, Self) {
        (x, Self { precision: x as u32 })
    }

    // Should NOT trigger - Self in nested generic
    fn nested_generic() -> Result<Option<Self>, String> {
        Ok(Some(Self { precision: 1 }))
    }

    // Should trigger - only uses primitive types
    fn helper(x: i32, y: i32, z: i32) -> i32 {
        //~^ ERROR: method `helper` has no relationship to `Self`
        x + y + z
    }

    // Should trigger - String operations, no Self
    fn format_string(s: &str) -> String {
        //~^ ERROR: method `format_string` has no relationship to `Self`
        format!("Formatted: {}", s)
    }
}

// Test with trait implementations
trait Display {
    fn display(&self);
}

impl Display for Calculator {
    // Should NOT trigger - trait implementation
    fn display(&self) {
        println!("Precision: {}", self.precision);
    }
}

// Test trait implementation without self parameter
trait UtilityTrait {
    fn process_data(x: i32, y: i32) -> i32;
}

struct UtilityImpl;

impl UtilityTrait for UtilityImpl {
    // Should NOT trigger - this is a trait implementation
    // Even though it has no Self relation, it's required by the trait
    fn process_data(x: i32, y: i32) -> i32 {
        x + y
    }
}

// Test with generic type
struct Container<T> {
    value: T,
}

impl<T> Container<T> {
    // Should trigger - no relationship to Self or T
    fn static_helper(x: i32, y: i32) -> i32 {
        //~^ ERROR: method `static_helper` has no relationship to `Self`
        x * y
    }

    // Should NOT trigger - returns Self
    fn new(value: T) -> Self {
        Self { value }
    }

    // Should NOT trigger - takes Self
    fn unwrap(self) -> T {
        self.value
    }

    // Should NOT trigger - takes &Self
    fn get(&self) -> &T {
        &self.value
    }
}

// Test with zero-sized type
struct Utility;

impl Utility {
    // Should trigger - no Self relation
    fn process(data: &str) -> String {
        //~^ ERROR: method `process` has no relationship to `Self`
        data.to_uppercase()
    }

    // Should NOT trigger - returns Self
    fn instance() -> Self {
        Self
    }
}

// Test with tuple struct
struct Point(i32, i32);

impl Point {
    // Should trigger - no Self relation
    fn distance_formula(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
        //~^ ERROR: method `distance_formula` has no relationship to `Self`
        (((x2 - x1).pow(2) + (y2 - y1).pow(2)) as f64).sqrt()
    }

    // Should NOT trigger - returns Self
    fn new(x: i32, y: i32) -> Self {
        Self(x, y)
    }

    // Should NOT trigger - takes &self
    fn x(&self) -> i32 {
        self.0
    }
}

// Test with array of Self
struct Matrix;

impl Matrix {
    // Should NOT trigger - returns array containing Self
    fn identity_matrices(count: usize) -> [Self; 3] {
        [Self, Self, Self]
    }

    // Should trigger - returns array of primitives
    fn numbers() -> [i32; 3] {
        //~^ ERROR: method `numbers` has no relationship to `Self`
        [1, 2, 3]
    }
}

// Test with function pointers
struct Callbacks;

impl Callbacks {
    // Should NOT trigger - function pointer references Self
    fn get_constructor() -> fn(i32) -> Self {
        |_| Self
    }

    // Should trigger - function pointer with no Self
    fn get_adder() -> fn(i32, i32) -> i32 {
        //~^ ERROR: method `get_adder` has no relationship to `Self`
        |a, b| a + b
    }
}

// Test async functions
struct AsyncStruct;

impl AsyncStruct {
    // Should trigger - async function with no Self relation
    async fn async_no_self(x: i32) -> i32 {
        //~^ ERROR: method `async_no_self` has no relationship to `Self`
        x + 1
    }

    // Should NOT trigger - async function taking Self parameter
    async fn async_consume(s: Self) -> i32 {
        42
    }

    // Should NOT trigger - async with self receiver
    async fn async_method(&self) -> i32 {
        42
    }
}

// Test impl Trait return types
struct ImplTraitStruct;

impl ImplTraitStruct {
    // Should trigger - impl Trait with no Self
    fn iter_no_self() -> impl Iterator<Item = i32> {
        //~^ ERROR: method `iter_no_self` has no relationship to `Self`
        std::iter::once(42)
    }

    // Should NOT trigger - impl Trait taking Self in parameter
    fn from_iter(items: impl Iterator<Item = Self>) -> Vec<Self> {
        items.collect()
    }
}

// Test raw struct name (should be treated as Self)
struct RawStructName {
    value: i32,
}

impl RawStructName {
    // Should NOT trigger - returns the raw struct type name
    fn create() -> RawStructName {
        RawStructName { value: 42 }
    }

    // Should NOT trigger - takes raw struct type name
    fn process(r: RawStructName) -> i32 {
        r.value
    }

    // Should NOT trigger - raw struct name in generic
    fn wrap() -> Option<RawStructName> {
        Some(RawStructName { value: 42 })
    }
}

// Test Box, Rc, and other smart pointers
use std::marker::PhantomData;
use std::rc::Rc;

struct SmartPointers;

impl SmartPointers {
    // Should NOT trigger - Box<Self>
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    // Should NOT trigger - Rc<Self>
    fn rced() -> Rc<Self> {
        Rc::new(Self)
    }

    // Should NOT trigger - takes Box<Self>
    fn unbox(b: Box<Self>) {
        drop(b);
    }

    // Should trigger - Box with no Self
    fn boxed_int() -> Box<i32> {
        //~^ ERROR: method `boxed_int` has no relationship to `Self`
        Box::new(42)
    }
}

// Test PhantomData
struct PhantomStruct<T>(PhantomData<T>);

impl<T> PhantomStruct<T> {
    // Should NOT trigger - PhantomData<Self>
    fn with_phantom() -> PhantomData<Self> {
        PhantomData
    }

    // Should trigger - PhantomData with different type
    fn phantom_other() -> PhantomData<i32> {
        //~^ ERROR: method `phantom_other` has no relationship to `Self`
        PhantomData
    }
}

// Test const generics
#[derive(Copy, Clone)]
struct ConstGenericStruct;

impl ConstGenericStruct {
    // Should NOT trigger - array with Self
    fn array_of_self<const N: usize>() -> [Self; N] {
        [Self; N]
    }

    // Should trigger - array with no Self
    fn array_of_int<const N: usize>() -> [i32; N] {
        //~^ ERROR: method `array_of_int` has no relationship to `Self`
        [0; N]
    }
}

// Test trait objects
struct TraitObjectStruct;

impl TraitObjectStruct {
    // Should NOT trigger - trait object taking Self
    fn callback_with_self() -> Box<dyn Fn(Self) -> i32> {
        Box::new(|_| 42)
    }

    // Should trigger - trait object with no Self
    fn callback_no_self() -> Box<dyn Fn(i32) -> i32> {
        //~^ ERROR: method `callback_no_self` has no relationship to `Self`
        Box::new(|x| x + 1)
    }
}

fn main() {}
