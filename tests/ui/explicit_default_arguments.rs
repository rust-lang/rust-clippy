// These names are confusing and unnecessary, I should probably change them
#![warn(clippy::explicit_default_arguments)]

use std::marker::PhantomData;

// Test types
struct DatabaseError;
struct NetworkError;
struct ComplexStruct<A, B, X, C, D>(PhantomData<(A, B, X, C, D)>);

// Type aliases with defaults
type DbResult<T = ()> = Result<T, DatabaseError>;
type NetResult<T = &'static str> = Result<T, NetworkError>;
type Optional<T = i64> = Option<T>;
type ComplexThing<A, B, C = u32, D = f64> = ComplexStruct<A, C, f32, B, D>;
type BoxedDefault<T = i128> = Box<T>;

// Module to test scoping
mod outer {
    pub type NestedResult<T = usize> = Result<T, ()>;
}

// Const declarations
const DB_CONST: DbResult<()> = Ok(());
const DB_OK: DbResult = Ok(());
const NET_CONST: NetResult<&str> = Ok("");
const NET_OK: NetResult = Ok("");

// Static declarations
static STATIC_DB: DbResult<()> = Ok(());
static STATIC_NET: NetResult = Ok("");
static OPTIONAL: Optional<i64> = Some(42);
static CUSTOM_OPT: Optional<f32> = Some(1.5);

// Associated types in traits
trait ExampleTrait1 {
    type AssocDb;
    type AssocNet;

    fn method() -> DbResult<()>;
}

trait ExampleTrait2<T> {
    type AssocTy1;
    type AssocTy2<F>;
}

impl ExampleTrait1 for () {
    type AssocDb = DbResult<()>;
    type AssocNet = NetResult;

    fn method() -> DbResult<()> {
        Ok(())
    }
}

// Function signatures
fn db_function(arg: DbResult<()>) -> DbResult<()>
where
    DbResult<()>: Send,
{
    arg
}

fn net_function(arg: NetResult) -> NetResult {
    arg
}

fn foo() -> ComplexThing<i8, i16> {
    todo!()
}

fn bar<T: ExampleTrait1>(val: T) -> T::AssocDb {
    todo!()
}
impl ComplexThing<DbResult<()>, ()> {
    const HELLO: usize = 5;
}
fn baz<T: ExampleTrait1>(val: T) -> [i32; <ComplexThing<DbResult<()>, ()>>::HELLO] {
    todo!()
}

fn quz<T: ExampleTrait2<i32>>() -> <T>::AssocTy1 {
    todo!()
}

fn qux<T: ExampleTrait2<i32>>() -> <T as ExampleTrait2<i32>>::AssocTy2<DbResult<&'static DbResult<()>>> {
    todo!()
}

// Struct fields
struct User {
    db_field: DbResult<()>,
    net_field: NetResult,
}

// Tuple struct
struct Response(DbResult<()>, NetResult);

// Enum variants
enum ApiResponse {
    Success(DbResult<()>),
    Failure(NetResult),
}

// Union fields
union DataHolder {
    db: std::mem::ManuallyDrop<DbResult<()>>,
    net: std::mem::ManuallyDrop<NetResult>,
}

// Type aliases

// Complex type scenarios
static COMPLEX_FULL: ComplexThing<i8, u8, u32, f64> = ComplexStruct(PhantomData);
static COMPLEX_PARTIAL: ComplexThing<i16, u16, u8> = ComplexStruct(PhantomData);

// Nested module type
static NESTED_RESULT: outer::NestedResult<usize> = Ok(42);

// Trait implementation with generics
impl<T> ExampleTrait1 for ComplexThing<DbResult<()>, T> {
    type AssocDb = DbResult<()>;
    type AssocNet = NetResult;

    fn method() -> DbResult<()> {
        Ok(())
    }
}

impl ExampleTrait1 for DbResult<()> {
    type AssocDb = DbResult<()>;
    type AssocNet = NetResult;

    fn method() -> DbResult<()> {
        Ok(())
    }
}

fn main() {
    // Local variables
    let a: DbResult<()> = Ok(());
    let b: NetResult = Ok("");

    // Function pointers
    let f: fn(DbResult<()>) -> DbResult<()> = db_function;

    // Expressions with std types
    let s = String::new();
    let v: Vec<String> = vec![s.clone()];
    let _o: Option<Vec<String>> = Some(v);

    // Box with default
    let boxed_int: BoxedDefault<i128> = Box::new(0);
    let boxed_float: BoxedDefault<f64> = Box::new(0.0);
}
