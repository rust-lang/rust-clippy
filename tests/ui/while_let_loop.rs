#![warn(clippy::while_let_loop)]
#![allow(clippy::uninlined_format_args)]
//@no-rustfix
fn main() {
    let y = Some(true);
    loop {
        //~^ while_let_loop

        if let Some(_x) = y {
            let _v = 1;
        } else {
            break;
        }
    }

    #[allow(clippy::never_loop)]
    loop {
        // no error, break is not in else clause
        if let Some(_x) = y {
            let _v = 1;
        }
        break;
    }

    loop {
        //~^ while_let_loop
        let Some(_x) = y else { break };
    }

    loop {
        //~^ while_let_loop
        let Some(_x) = y else {
            let _z = 1;
            break;
        };
    }

    loop {
        //~^ while_let_loop

        match y {
            Some(_x) => true,
            None => break,
        };
    }

    loop {
        //~^ while_let_loop

        let x = match y {
            Some(x) => x,
            None => break,
        };
        let _x = x;
        let _str = "foo";
    }

    loop {
        //~^ while_let_loop

        let x = match y {
            Some(x) => x,
            None => break,
        };
        {
            let _a = "bar";
        };
        {
            let _b = "foobar";
        }
    }

    loop {
        // no error, hoisting from match arms isn't supported
        match y {
            Some(_x) => true,
            _ => {
                let _z = 1;
                break;
            },
        };
    }

    while let Some(x) = y {
        // no error, obviously
        println!("{}", x);
    }

    // #675, this used to have a wrong suggestion
    loop {
        //~^ while_let_loop

        let (e, l) = match "".split_whitespace().next() {
            Some(word) => (word.is_empty(), word.len()),
            None => break,
        };

        let _ = (e, l);
    }
}

fn issue771() {
    let mut a = 100;
    let b = Some(true);
    loop {
        if a > 10 {
            break;
        }

        match b {
            Some(_) => a = 0,
            None => break,
        }
    }
}

fn issue1017() {
    let r: Result<u32, u32> = Ok(42);
    let mut len = 1337;

    loop {
        match r {
            Err(_) => len = 0,
            Ok(length) => {
                len = length;
                break;
            },
        }
    }
}

#[allow(clippy::never_loop)]
fn issue1948() {
    // should not trigger clippy::while_let_loop lint because break passes an expression
    let a = Some(10);
    let b = loop {
        if let Some(c) = a {
            break Some(c);
        } else {
            break None;
        }
    };
}

fn issue_7913(m: &std::sync::Mutex<Vec<u32>>) {
    // Don't lint. The lock shouldn't be held while printing.
    loop {
        let x = if let Some(x) = m.lock().unwrap().pop() {
            x
        } else {
            break;
        };

        println!("{}", x);
    }
}

fn issue_5715(mut m: core::cell::RefCell<Option<u32>>) {
    // Don't lint. The temporary from `borrow_mut` must be dropped before overwriting the `RefCell`.
    loop {
        let x = if let &mut Some(x) = &mut *m.borrow_mut() {
            x
        } else {
            break;
        };

        m = core::cell::RefCell::new(Some(x + 1));
    }
}

mod issue_362 {
    pub fn merge_sorted<T>(xs: Vec<T>, ys: Vec<T>) -> Vec<T>
    where
        T: PartialOrd,
    {
        let total_len = xs.len() + ys.len();
        let mut res = Vec::with_capacity(total_len);
        let mut ix = xs.into_iter().peekable();
        let mut iy = ys.into_iter().peekable();
        loop {
            //~^ while_let_loop
            let lt = match (ix.peek(), iy.peek()) {
                (Some(x), Some(y)) => x < y,
                _ => break,
            };
            res.push(if lt { &mut ix } else { &mut iy }.next().unwrap());
        }
        res.extend(ix);
        res.extend(iy);
        res
    }
}

fn let_assign() {
    loop {
        //~^ while_let_loop
        let x = if let Some(y) = Some(3) {
            y
        } else {
            break;
        };
        if x == 3 {
            break;
        }
    }

    loop {
        //~^ while_let_loop
        let x: u32 = if let Some(y) = Some(3) {
            y
        } else {
            break;
        };
        if x == 3 {
            break;
        }
    }

    loop {
        //~^ while_let_loop
        let x = if let Some(x) = Some(3) {
            x
        } else {
            break;
        };
        if x == 3 {
            break;
        }
    }

    loop {
        //~^ while_let_loop
        let x: u32 = if let Some(x) = Some(3) {
            x
        } else {
            break;
        };
        if x == 3 {
            break;
        }
    }

    loop {
        //~^ while_let_loop
        let x = if let Some(x) = Some(2) {
            let t = 1;
            t + x
        } else {
            break;
        };
        if x == 3 {
            break;
        }
    }
}

fn issue16378() {
    loop {
        //~^ while_let_loop
        let Some(x) = std::hint::black_box(None::<i32>) else {
            println!("fail");
            break;
        };
        println!("x = {x}");
    }
}

fn hoist_with_multiple_stmts() {
    let y = Some(true);
    loop {
        //~^ while_let_loop
        let Some(x) = y else {
            let a = 1;
            let b = 2;
            let _c = a + b;
            println!("sum: {}", a + b);
            eprintln!("failed");
            break;
        };
        println!("x = {x}");
    }
}

fn hoist_with_semicolon_less_stmt() {
    let y = Some(true);
    loop {
        //~^ while_let_loop
        let Some(x) = y else {
            if std::hint::black_box(true) {
                println!("pass");
            }
            match 42 {
                0 => println!("zero"),
                _ => println!("non-zero"),
            }
            break;
        };
        println!("x = {x}");
    }
}

fn hoist_with_return() -> Option<i32> {
    loop {
        //~^ while_let_loop
        let Some(x) = std::hint::black_box(None::<i32>) else {
            if true {
                return None;
            }
            break;
        };
        println!("x = {x}");
    }
    Some(42)
}

fn hoist_with_labeled_break() {
    'outer: loop {
        loop {
            //~^ while_let_loop
            let Some(x) = std::hint::black_box(None::<i32>) else {
                if true {
                    break 'outer;
                }
                break;
            };
            println!("x = {x}");
        }
    }
}

fn hoist_with_labeled_continue() {
    'outer: loop {
        loop {
            //~^ while_let_loop
            let Some(x) = std::hint::black_box(None::<i32>) else {
                if true {
                    continue 'outer;
                }
                break;
            };
            println!("x = {x}");
        }
        break;
    }
}

fn hoist_with_label_on_transformed_loop() {
    let y = Some(true);
    'my_loop: loop {
        //~^ while_let_loop
        let Some(x) = y else {
            println!("done");
            break;
        };
        println!("x = {x}");
    }
}

fn no_hoist_break_targets_transformed_loop() {
    // Should NOT lint: hoisted stmt contains a break targeting the loop being transformed
    loop {
        let Some(x) = std::hint::black_box(None::<i32>) else {
            if true {
                break;
            }
            println!("msg");
            break;
        };
        println!("x = {x}");
    }
}

fn no_hoist_continue_targets_transformed_loop() {
    // no error, unlabeled continue targets the loop being transformed
    loop {
        let Some(x) = std::hint::black_box(None::<i32>) else {
            if true {
                continue;
            }
            break;
        };
        println!("x = {x}");
    }
}

fn no_hoist_labeled_break_targets_transformed_loop() {
    // no error, labeled break targets the loop being transformed
    'my_loop: loop {
        let Some(x) = std::hint::black_box(None::<i32>) else {
            if true {
                break 'my_loop;
            }
            break;
        };
        println!("x = {x}");
    }
}

fn no_hoist_break_with_value() {
    // no error, break with a value is not a simple break
    let _result = loop {
        let Some(x) = std::hint::black_box(None::<i32>) else {
            break 42;
        };
        println!("x = {x}");
    };
}

fn hoist_with_nested_inner_loop() {
    loop {
        //~^ while_let_loop
        let Some(x) = std::hint::black_box(None::<i32>) else {
            for i in 0..3 {
                if i == 1 {
                    break;
                }
                println!("{i}");
            }
            break;
        };
        println!("x = {x}");
    }
}
