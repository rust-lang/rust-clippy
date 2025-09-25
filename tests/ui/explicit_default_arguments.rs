#![warn(clippy::explicit_default_arguments)]

use std::marker::PhantomData;

// Test types
struct DatabaseError;
struct NetworkError;
struct ComplexStruct<A, B, C, D>(PhantomData<(A, B, C, D)>);

// Type aliases with defaults
type DbResult<T = ()> = Result<T, DatabaseError>;
type NetResult<T = &'static str> = Result<T, NetworkError>;
type Optional<T = i64> = Option<T>;
type ComplexThing<A, B, C = u32, D = f64> = ComplexStruct<A, B, C, D>;
type BoxedDefault<T = i128> = Box<T>;

// Module to test scoping
mod outer {
    pub type NestedResult<T = usize> = Result<T, ()>;
}

// Const declarations
const DB_CONST: DbResult<()> = Ok(());
//~^ explicit_default_arguments
const DB_OK: DbResult = Ok(());
const NET_CONST: NetResult<&str> = Ok("");
//~^ explicit_default_arguments
const NET_OK: NetResult = Ok("");

// Static declarations
static STATIC_DB: DbResult<()> = Ok(());
//~^ explicit_default_arguments
static STATIC_NET: NetResult = Ok("");
static OPTIONAL: Optional<i64> = Some(42);
//~^ explicit_default_arguments
static CUSTOM_OPT: Optional<f32> = Some(1.5);

// Associated types in traits
trait ExampleTrait {
    type AssocDb;
    type AssocNet;

    fn method() -> DbResult<()>;
    //~^ explicit_default_arguments
}

impl ExampleTrait for () {
    type AssocDb = DbResult<()>;
    //~^ explicit_default_arguments
    type AssocNet = NetResult;

    fn method() -> DbResult<()> {
        //~^ explicit_default_arguments
        Ok(())
    }
}

// Function signatures
fn db_function(arg: DbResult<()>) -> DbResult<()> {
    //~^ explicit_default_arguments
    //~| explicit_default_arguments
    arg
}

fn net_function(arg: NetResult) -> NetResult {
    arg
}

// Struct fields
struct User {
    db_field: DbResult<()>,
    //~^ explicit_default_arguments
    net_field: NetResult,
}

// Tuple struct
struct Response(
    DbResult<()>,
    //~^ explicit_default_arguments
    NetResult,
);

// Enum variants
enum ApiResponse {
    Success(DbResult<()>),
    //~^ explicit_default_arguments
    Failure(NetResult),
}

// Union fields
union DataHolder {
    db: std::mem::ManuallyDrop<DbResult<()>>,
    //~^ explicit_default_arguments
    net: std::mem::ManuallyDrop<NetResult>,
}

// Type aliases
type DbAlias = DbResult<()>;
//~^ explicit_default_arguments
type NetAlias = NetResult;

// Complex type scenarios
static COMPLEX_FULL: ComplexThing<i8, u8, u32, f64> = ComplexStruct(PhantomData);
//~^ explicit_default_arguments
static COMPLEX_PARTIAL: ComplexThing<i16, u16, u8> = ComplexStruct(PhantomData);

// Nested module type
static NESTED_RESULT: outer::NestedResult<usize> = Ok(42);
//~^ explicit_default_arguments

// Trait implementation with generics
impl<T> ExampleTrait for ComplexThing<T, T> {
    type AssocDb = DbResult<()>;
    //~^ explicit_default_arguments
    type AssocNet = NetResult;

    fn method() -> DbResult<()> {
        //~^ explicit_default_arguments
        Ok(())
    }
}

fn main() {
    // Local variables
    let a: DbResult<()> = Ok(());
    //~^ explicit_default_arguments
    let b: NetResult = Ok("");

    // Function pointers
    let f: fn(DbResult<()>) -> DbResult<()> = db_function;
    //~^ explicit_default_arguments
    //~| explicit_default_arguments

    // Expressions with std types
    let s = String::new();
    let v: Vec<String> = vec![s.clone()];
    let _o: Option<Vec<String>> = Some(v);

    // Box with default
    let boxed_int: BoxedDefault<i128> = Box::new(0);
    //~^ explicit_default_arguments
    let boxed_float: BoxedDefault<f64> = Box::new(0.0);
}
