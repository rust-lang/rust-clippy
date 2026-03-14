#![warn(clippy::manual_pop_if)]
#![allow(clippy::collapsible_if, clippy::redundant_closure)]
//@no-rustfix

fn is_some_and_pattern(mut vec: Vec<i32>) {
    if false {
        // something
    } else if vec.last().is_some_and(|x| *x > 2) {
        //~^ manual_pop_if
        vec.pop().unwrap();
    }

    if vec.last().is_some_and(|x| *x > 2) {
        //~^ manual_pop_if
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    if vec.last().is_some_and(|x| *x > 2) {
        //~^ manual_pop_if
        println!("Popped: {}", vec.pop().unwrap());
    }
}

fn if_let_pattern(mut vec: Vec<i32>) {
    if let Some(x) = vec.last() {
        //~^ manual_pop_if
        if *x > 2 {
            let val = vec.pop().unwrap();
            println!("Popped: {}", val);
        }
    }

    if let Some(x) = vec.last() {
        //~^ manual_pop_if
        if *x > 2 {
            println!("Popped: {}", vec.pop().unwrap());
        }
    }
}

fn let_chain_pattern(mut vec: Vec<i32>) {
    if let Some(x) = vec.last()
        //~^ manual_pop_if
        && *x > 2
    {
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    if let Some(x) = vec.last()
        //~^ manual_pop_if
        && *x > 2
    {
        println!("Popped: {}", vec.pop().unwrap());
    }
}

fn map_unwrap_or_pattern(mut vec: Vec<i32>) {
    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        //~^ manual_pop_if
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        //~^ manual_pop_if
        println!("Popped: {}", vec.pop().unwrap());
    }
}
