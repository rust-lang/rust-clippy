#![warn(clippy::fn_to_numeric_cast_any)]
#![expect(clippy::fn_to_numeric_cast, clippy::fn_to_numeric_cast_with_truncation)]
#![allow(function_casts_as_integer)]
//@no-rustfix
fn foo() -> u8 {
    0
}

fn generic_foo<T>(x: T) -> T {
    x
}

trait Trait {
    fn static_method() -> u32 {
        2
    }
}

struct Struct;

impl Trait for Struct {}

fn fn_pointer_to_integer() {
    let _ = foo as i8;
    //~^ fn_to_numeric_cast_any

    let _ = foo as i16;
    //~^ fn_to_numeric_cast_any

    let _ = foo as i32;
    //~^ fn_to_numeric_cast_any

    let _ = foo as i64;
    //~^ fn_to_numeric_cast_any

    let _ = foo as i128;
    //~^ fn_to_numeric_cast_any

    let _ = foo as isize;
    //~^ fn_to_numeric_cast_any

    let _ = foo as u8;
    //~^ fn_to_numeric_cast_any

    let _ = foo as u16;
    //~^ fn_to_numeric_cast_any

    let _ = foo as u32;
    //~^ fn_to_numeric_cast_any

    let _ = foo as u64;
    //~^ fn_to_numeric_cast_any

    let _ = foo as u128;
    //~^ fn_to_numeric_cast_any

    let _ = foo as usize;
    //~^ fn_to_numeric_cast_any
}

fn static_method_to_integer() {
    let _ = Struct::static_method as usize;
    //~^ fn_to_numeric_cast_any
}

fn fn_with_fn_arg(f: fn(i32) -> u32) -> usize {
    f as usize
    //~^ fn_to_numeric_cast_any
}

fn fn_with_generic_static_trait_method<T: Trait>() -> usize {
    T::static_method as usize
    //~^ fn_to_numeric_cast_any
}

fn closure_to_fn_to_integer() {
    let clos = |x| x * 2_u32;

    let _ = (clos as fn(u32) -> u32) as usize;
    //~^ fn_to_numeric_cast_any
}

fn fn_to_raw_ptr() {
    let _ = foo as *const ();
    let _ = foo as *mut ();
}

fn cast_fn_to_self() {
    // Casting to the same function pointer type should be permitted.
    let _ = foo as fn() -> u8;
}

fn cast_generic_to_concrete() {
    // Casting to a more concrete function pointer type should be permitted.
    let _ = generic_foo as fn(usize) -> usize;
}

fn cast_closure_to_fn() {
    // Casting a closure to a function pointer should be permitted.
    let id = |x| x;
    let _ = id as fn(usize) -> usize;
}

mod issue18087 {
    fn unit_fn() {}
    fn ret_string() -> String {
        String::new()
    }
    fn ret_bool() -> bool {
        true
    }
    fn ret_char() -> char {
        'a'
    }
    fn ret_f32() -> f32 {
        0.0
    }
    fn ret_thin_ptr() -> *const u8 {
        std::ptr::null()
    }
    fn ret_fat_ptr() -> *const [u8] {
        &[]
    }
    unsafe fn unsafe_fn() -> u8 {
        0
    }

    enum UnitEnum {
        A,
    }
    enum DropEnum {
        A,
    }
    impl Drop for DropEnum {
        fn drop(&mut self) {}
    }
    enum TupleEmptyEnum {
        A(),
    }

    fn ret_unit_enum() -> UnitEnum {
        UnitEnum::A
    }
    fn ret_drop_enum() -> DropEnum {
        DropEnum::A
    }
    fn ret_tuple_empty_enum() -> TupleEmptyEnum {
        TupleEmptyEnum::A()
    }

    trait Assoc {
        type Out;
        fn make() -> Self::Out;
    }
    impl Assoc for () {
        type Out = u8;
        fn make() -> u8 {
            0
        }
    }

    struct FnHolder {
        f: fn() -> u8,
    }

    fn test<T: Assoc>(r: &fn() -> u8, h: FnHolder) {
        // Should lint without invalid `unit_fn() as usize` suggestion (E0605).
        assert_eq!(unit_fn as *const u8 as usize, unit_fn as usize);
        //~^ fn_to_numeric_cast_any

        let _ = ret_string as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_fat_ptr as usize;
        //~^ fn_to_numeric_cast_any
        let _ = unsafe_fn as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_drop_enum as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_tuple_empty_enum as usize;
        //~^ fn_to_numeric_cast_any
        let _ = T::make as usize;
        //~^ fn_to_numeric_cast_any

        // Should suggest invoking the function when valid.
        let _ = ret_bool as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_char as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_f32 as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_thin_ptr as usize;
        //~^ fn_to_numeric_cast_any
        let _ = ret_unit_enum as usize;
        //~^ fn_to_numeric_cast_any
        let _ = <() as Assoc>::make as usize;
        //~^ fn_to_numeric_cast_any
        let _ = *r as usize;
        //~^ fn_to_numeric_cast_any
        let _ = h.f as usize;
        //~^ fn_to_numeric_cast_any

        let clos = || 2_u32;
        let _ = (clos as fn() -> u32) as usize;
        //~^ fn_to_numeric_cast_any

        macro_rules! get_field {
            ($h:expr) => {
                $h.f
            };
        }
        let _ = get_field!(h) as usize;
        //~^ fn_to_numeric_cast_any
    }

    async fn get_fn() -> fn() -> u8 {
        super::foo
    }

    async fn test_async() {
        let _ = get_fn().await as usize;
        //~^ fn_to_numeric_cast_any
    }
}

fn main() {}
