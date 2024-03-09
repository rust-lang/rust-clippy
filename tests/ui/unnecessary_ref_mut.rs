#![warn(clippy::unnecessary_ref_mut)]
#![allow(clippy::disallowed_names, clippy::single_match)]
#![no_main]

struct Foo;
impl Foo {
    fn immutable(&self, s: &str) {}

    fn mutable(&self, s: &mut str) {}
}

struct Config {
    name: String,
}

fn let_some() {
    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            //~^ ERROR: unnecessary ref mut
            return;
        };

        s_ref.as_bytes();
    }

    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            return;
        };

        s_ref.push('A');
    }
}

fn if_let_some() {
    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            //~^ ERROR: unnecessary ref mut
            s_ref.as_str();
        };
    }

    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            s_ref.push('A');
        };
    }
}

fn match_expr() {
    {
        let mut s = Some(String::new());
        match s {
            Some(ref mut s_ref) => {
                //~^ ERROR: unnecessary ref mut
                s_ref.as_bytes();
            },
            None => {},
        }
    }

    {
        let mut s = Some(String::new());
        match s {
            Some(ref mut s_ref) => {
                s_ref.push('A');
            },
            None => {},
        }
    }
}

fn bind_split_field() {
    {
        let mut config = Config {
            name: "name".to_string(),
        };
        let Config { ref mut name } = config;
        //~^ ERROR: unnecessary ref mut
        name.to_string();
    }

    {
        let mut config = Config {
            name: "name".to_string(),
        };
        let Config { ref mut name } = config;
        name.push('A');
    }
}

fn fn_call_args() {
    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            //~^ ERROR: unnecessary ref mut
            return;
        };

        fn call(f: &str) {}
        call(s_ref);
    }

    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            return;
        };

        fn call(f: &mut str) {}
        call(s_ref);
    }
}

fn method_call_args() {
    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            //~^ ERROR: unnecessary ref mut
            return;
        };

        let foo = Foo;
        foo.immutable(s_ref);
    }

    {
        let mut s = Some(String::new());
        let Some(ref mut s_ref) = s else {
            return;
        };

        let foo = Foo;
        foo.mutable(s_ref);
    }
}

#[allow(static_mut_refs)]
fn binding() {
    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            //~^ ERROR: unnecessary ref mut
            let s_ref2 = s_ref;
            s_ref2.as_str();
        };
    }

    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            //~^ ERROR: unnecessary ref mut
            let _ = s_ref;
        };
    }

    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            let s_ref2 = s_ref;
            s_ref2.push('A');
        };
    }

    {
        let mut str = "".to_string();
        let mut s = Some(String::new());
        let mut s2 = &mut str;
        if let Some(ref mut s_ref) = s {
            s2 = s_ref;
            s2.push('A');
        };
    }

    static mut STR: Option<usize> = Some(0);
    static mut OUTSIDE: Option<&mut usize> = None;

    unsafe {
        if let Some(ref mut s_ref) = STR {
            OUTSIDE = Some(s_ref);
        }
    }
}

fn binding_tuple_in_variant() {
    {
        let s = String::new();
        if let Some((_, ref mut s_ref)) = Some(((), s)) {
            //~^ ERROR: unnecessary ref mut
            s_ref.as_str();
        };
    }

    {
        let s = String::new();
        if let Some((_, ref mut s_ref)) = Some(((), s)) {
            s_ref.push('A');
        };
    }
}

fn assign() {
    {
        let mut s = Some(String::new());
        if let Some(ref mut s_ref) = s {
            *s_ref = "".to_string();
        };
    }
}
