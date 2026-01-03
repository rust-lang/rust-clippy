#![warn(clippy::explicit_default_arguments)]

// ============================================================================
// Test 1: single default parameter
// ============================================================================

type Result<T = ()> = std::result::Result<T, String>;

fn basic_trigger() -> Result<()> {
    //~^ explicit_default_arguments
    Ok(())
}

fn basic_ok_implicit() -> Result {
    Ok(())
}

fn basic_ok_non_default() -> Result<i32> {
    Ok(5)
}

// ============================================================================
// Test 2: multiple type parameters with defaults
// ============================================================================

type Multi<T = i32, U = String> = std::result::Result<T, U>;

fn multi_all_defaults() -> Multi<i32, String> {
    //~^ explicit_default_arguments
    //~| explicit_default_arguments
    Ok(5)
}

fn multi_partial() -> Multi<i32> {
    //~^ explicit_default_arguments
    Ok(5)
}

fn multi_none() -> Multi {
    Ok(5)
}

// ============================================================================
// Test 3: complex default expressions
// ============================================================================

type Complex<T = std::collections::HashMap<String, Vec<i32>>> = Option<T>;

fn complex_default() -> Complex<std::collections::HashMap<String, Vec<i32>>> {
    //~^ explicit_default_arguments
    None
}

fn complex_implicit() -> Complex {
    None
}

// ============================================================================
// Test 4: trait implementations
// ============================================================================

trait MyTrait1 {}
trait MyTrait2 {}

impl MyTrait1 for Result<()> {
    //~^ explicit_default_arguments
}

impl MyTrait2 for Result {}

// ============================================================================
// Test 5: type alias definitions using types with defaults
// ============================================================================

type AliasWithDefault = Result<()>;
//~^ explicit_default_arguments
type AliasImplicit = Result;

type AliasNonDefault = Result<String>;

// ============================================================================
// Test 6: defaults that are type aliases themselves
// ============================================================================

type DefaultType = i32;
type UsingDefault<T = DefaultType> = Option<T>;

fn alias_default() -> UsingDefault<DefaultType> {
    //~^ explicit_default_arguments
    None
}

fn alias_implicit() -> UsingDefault {
    None
}

// ============================================================================
// Test 7: macro usage
// ============================================================================

macro_rules! def_foo {
    () => {
        let foo: Result<()> = Ok(());
    };
}

fn main() {
    // Should NOT trigger: macro expansion
    def_foo!();
}
