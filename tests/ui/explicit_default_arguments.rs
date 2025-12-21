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
    type AssocTy;
    type AssocNet;

    fn method(&self) -> DbResult<()>;
}

trait ExampleTrait2<T> {
    type AssocTy1;
    type AssocTy2<F>;
}

impl ExampleTrait1 for () {
    type AssocTy = DbResult<()>;
    type AssocNet = NetResult;

    fn method(&self) -> DbResult<()> {
        Ok(())
    }
}

impl ExampleTrait2<DbResult<()>> for () {
    type AssocTy1 = DbResult<DbResult<()>>;
    type AssocTy2<T> = DbResult<()>;
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

trait ObjSafe<A, B, C, D, E> {
    type AssocTy1;
    type AssocTy2;
    type AssocTy3;
    type AssocTy4;
}
// fn foo() -> ComplexThing<i8, i16> {
//     todo!()
// }
fn foo<T: ExampleTrait1<AssocTy = i32>>(
    _hello: Box<dyn ExampleTrait1<AssocTy = i32, AssocNet = DbResult<()>> + >,
) -> impl ExampleTrait2<DbResult<()>, AssocTy1 = DbResult<DbResult<()>>, AssocTy2<DbResult<()>> = DbResult<()>>
where
    i32: Send
{
}

fn bar<T: ExampleTrait2<DbResult<()>>>(val: T) -> T::AssocTy<DbResult<()>> {
    todo!()
}
impl ComplexThing<DbResult<()>, ()> {
    const HELLO: usize = 5;
}
fn baz<T: ExampleTrait1>(val: T) -> [i32; <ComplexThing<DbResult<()>, ()>>::HELLO] {
    todo!()
}

fn quz() -> impl ExampleTrait2<DbResult<()>>
{

}

fn qux<T: ExampleTrait2<DbResult<()>>>() -> <T as ExampleTrait2<DbResult<()>>>::AssocTy2<DbResult<&'static DbResult<()>>> {
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

struct Random<T = DbResult<()>> {
    val: T,
}

// Type aliases

// Complex type scenarios
static COMPLEX_FULL: ComplexThing<i8, u8, u32, f64> = ComplexStruct(PhantomData);
static COMPLEX_PARTIAL: ComplexThing<i16, u16, u8> = ComplexStruct(PhantomData);

// Nested module type
static NESTED_RESULT: outer::NestedResult<usize> = Ok(42);

// Trait implementation with generics
impl<T> ExampleTrait1 for ComplexThing<DbResult<()>, T> {
    type AssocTy = DbResult<()>;
    type AssocNet = NetResult;

    fn method(&self) -> DbResult<()> {
        Ok(())
    }
}

impl ExampleTrait1 for DbResult<()> {
    type AssocTy = DbResult<()>;
    type AssocNet = NetResult;

    fn method(&self) -> DbResult<()> {
        Ok(())
    }
}

fn with_generics<T>(val: T, other: u32) {}

struct Pat<T> {
    field: Option<T>
}
impl<T> Pat<T> {
    fn new<A>() {}
}

enum Other<A, B, C, D> {
    One(A, B, C, D, u8),
}

fn has_pattern(Pat::<i32> { field }: Pat<i32>) {

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

    <Pat<i32>>::new::<i32>();
    with_generics::<i32>(10, 0);
    Ok(()) as DbResult<()>;

    let expr = [1; 5];

    if let Other::One::<i8, i16, i32, i64>(0, 1,2, 3, 4) = Other::One::<i8, i16, i32, i64>(0, 1, 2, 3, 4) {

    }
    if let Pat::<i32>{ field: Some::<i32>(10) } = (Pat { field: Some::<i32>(10)}) {

    }
    let vec: Vec<i32> = vec![];
    if let arr @ [_, ..] = vec.as_slice() {

    }
}
