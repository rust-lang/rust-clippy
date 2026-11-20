//@compile-flags: -Zpolonius=off
//@no-rustfix: has placeholders
#![warn(clippy::unnecessary_unwrap)]

// Borrowing the scrutinee (`&self.field`) evaluates the borrow before the match, so under NLL it
// can stay live past the `if` and conflict with a later mutation (#16182). With polonius disabled
// the suggestion therefore falls back to a `ref`/`ref mut` binding, which borrows only the matched
// field value. See `simple_conditionals.rs` for the polonius-enabled suggestions.

mod issue16182 {
    struct Wrapper {
        option: Option<String>,
        result: Result<String, String>,
    }

    impl Wrapper {
        fn option_ref(&mut self) -> &String {
            if self.option.is_some() {
                return self.option.as_ref().unwrap();
                //~^ unnecessary_unwrap
            }
            self.option.insert(String::new())
        }

        fn option_mut(&mut self) -> &mut String {
            if self.option.is_some() {
                return self.option.as_mut().unwrap();
                //~^ unnecessary_unwrap
            }
            self.option.insert(String::new())
        }

        fn result_ok(&mut self) -> &String {
            if self.result.is_ok() {
                return self.result.as_ref().unwrap();
                //~^ unnecessary_unwrap
            }
            self.result = Ok(String::new());
            match &self.result {
                Ok(value) => value,
                Err(_) => unreachable!(),
            }
        }

        fn result_err(&mut self) -> &String {
            if self.result.is_err() {
                return self.result.as_ref().unwrap_err();
                //~^ unnecessary_unwrap
            }
            self.result = Err(String::new());
            match &self.result {
                Ok(_) => unreachable!(),
                Err(error) => error,
            }
        }

        fn suggested_option_ref(&mut self) -> &String {
            if let Some(ref value) = self.option {
                return value;
            }
            self.option.insert(String::new())
        }

        fn suggested_option_mut(&mut self) -> &mut String {
            if let Some(ref mut value) = self.option {
                return value;
            }
            self.option.insert(String::new())
        }

        fn suggested_result_ok(&mut self) -> &String {
            if let Ok(ref value) = self.result {
                return value;
            }
            self.result = Ok(String::new());
            match &self.result {
                Ok(value) => value,
                Err(_) => unreachable!(),
            }
        }

        fn suggested_result_err(&mut self) -> &String {
            if let Err(ref error) = self.result {
                return error;
            }
            self.result = Err(String::new());
            match &self.result {
                Ok(_) => unreachable!(),
                Err(error) => error,
            }
        }
    }
}

fn main() {}
