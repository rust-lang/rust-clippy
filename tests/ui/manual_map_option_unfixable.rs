//! In the following cases, the suggestion is incorrect due to coercion

//@no-rustfix
#![warn(clippy::manual_map)]

fn main() {}

fn issue7077() {
    let s = &String::new();
    #[allow(clippy::needless_match)]
    let _: Option<&str> = match Some(s) {
        //~^ manual_map
        Some(s) => Some(s),
        None => None,
    };
}

fn pull_12712() {
    let x: Option<Vec<&[u8]>> = match Some(()) {
        //~^ manual_map
        Some(_) => Some(vec![b"1234"]),
        None => None,
    };

    // type of `s` is coerced from `&String` to `&str`
    let s = Some(String::new());
    let x: Option<(String, &str)> = match &s {
        //~^ manual_map
        Some(x) => Some({ if let Some(ref s) = s { (x.clone(), s) } else { panic!() } }),
        None => None,
    };
}

// issue #12659
mod with_type_coercion {
    trait DummyTrait {}

    fn foo<T: DummyTrait, F: Fn() -> Result<T, ()>>(f: F) {
        let _: Option<Result<Box<dyn DummyTrait>, ()>> = match Some(0) {
            //~^ manual_map
            Some(_) => Some(match f() {
                Ok(res) => Ok(Box::new(res)),
                _ => Err(()),
            }),
            None => None,
        };

        let _: Option<Box<&[u8]>> = match Some(()) {
            //~^ manual_map
            Some(_) => Some(Box::new(b"1234")),
            None => None,
        };

        let x = String::new();
        let _: Option<Box<&str>> = match Some(()) {
            //~^ manual_map
            Some(_) => Some(Box::new(&x)),
            None => None,
        };

        let _: Option<&str> = match Some(()) {
            //~^ manual_map
            Some(_) => Some(&x),
            None => None,
        };
    }

    #[allow(clippy::redundant_allocation)]
    fn bar() {
        fn f(_: Option<Box<&[u8]>>) {}

        let x: &[u8; 4] = b"1234";
        f(match Some(()) {
            //~^ manual_map
            Some(_) => Some(Box::new(x)),
            None => None,
        });
    }

    fn with_fn_ret(s: &Option<String>) -> Option<(String, &str)> {
        // `map` doesn't work as the return type is adjusted.
        match s {
            //~^ manual_map
            Some(x) => Some({ if let Some(s) = s { (x.clone(), s) } else { panic!() } }),
            None => None,
        }
    }

    fn with_fn_ret_2(s: &Option<String>) -> Option<(String, &str)> {
        if true {
            // `map` doesn't work as the return type is adjusted.
            return match s {
                //~^ manual_map
                Some(x) => Some({ if let Some(s) = s { (x.clone(), s) } else { panic!() } }),
                None => None,
            };
        }
        None
    }

    #[allow(clippy::needless_late_init)]
    fn with_fn_ret_3<'a>(s: &'a Option<String>) -> Option<(String, &'a str)> {
        let x: Option<(String, &'a str)>;
        x = {
            match s {
                //~^ manual_map
                Some(x) => Some({ if let Some(s) = s { (x.clone(), s) } else { panic!() } }),
                None => None,
            }
        };
        x
    }
}

pub fn issue14389(opt: Option<&'static i32>) -> Option<&'static i32> {
    if let Some(o) = opt { Some(o) } else { None }
    //~^ manual_map
}
