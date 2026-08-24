#![warn(clippy::manual_pop_if)]
#![expect(clippy::collapsible_if, clippy::needless_borrow)]

//@no-rustfix

fn main() {}

struct Wrapper {
    vec: Vec<i32>,
}

fn is_some_and_pattern(mut vec: Vec<i32>) {
    if false {
        // something
    } else if vec.last().is_some_and(|x| *x > 2) {
        vec.pop().unwrap();
    }
    //~^^^ manual_pop_if

    //~v manual_pop_if
    if vec.last().is_some_and(|x| *x > 2) {
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|x| *x > 2) {
        println!("Popped: {}", vec.pop().unwrap());
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|x| *x > 2) {
        // a comment before the pop
        vec.pop().unwrap();
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|x| *x > 2) {
        vec.pop().unwrap();
        // a comment after the pop
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|x| vec.len() > 1 && *x > 10) {
        vec.pop().unwrap();
    }

    let r = &vec;
    //~v manual_pop_if
    if vec.last().is_some_and(|x| r.len() > 1 && *x > 10) {
        vec.pop().unwrap();
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|ref x| **x > 10) {
        vec.pop().unwrap();
    }

    //~v manual_pop_if
    if vec.last().is_some_and(|mut x| {
        x = &20;
        *x > 10
    }) {
        vec.pop().unwrap();
    }
}

fn if_let_pattern(mut vec: Vec<i32>) {
    //~v manual_pop_if
    if let Some(x) = vec.last() {
        if *x > 2 {
            let val = vec.pop().unwrap();
            println!("Popped: {}", val);
        }
    }

    //~v manual_pop_if
    if let Some(x) = vec.last() {
        if *x > 2 {
            println!("Popped: {}", vec.pop().unwrap());
        }
    }

    //~v manual_pop_if
    if let Some(x) = vec.last() {
        if *x > 2 {
            // a comment before the pop
            vec.pop().unwrap();
        }
    }

    //~v manual_pop_if
    if let Some(x) = vec.last() {
        if *x > 2 {
            vec.pop().unwrap();
            // a comment after the pop
        }
    }

    if let Some(x) = vec.last() {
        if vec.len() > 1 && *x > 10 {
            //~^ manual_pop_if
            vec.pop().unwrap();
        }
    }

    let r = &vec;
    //~v manual_pop_if
    if let Some(x) = vec.last() {
        if r.len() > 1 && *x > 10 {
            vec.pop().unwrap();
        }
    }
}

fn let_chain_pattern(mut vec: Vec<i32>) {
    //~v manual_pop_if
    if let Some(x) = vec.last()
        && *x > 2
    {
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    //~v manual_pop_if
    if let Some(x) = vec.last()
        && *x > 2
    {
        println!("Popped: {}", vec.pop().unwrap());
    }

    //~v manual_pop_if
    if let Some(x) = vec.last()
        && *x > 2
    {
        // a comment before the pop
        vec.pop().unwrap();
    }

    //~v manual_pop_if
    if let Some(x) = vec.last()
        && *x > 2
    {
        vec.pop().unwrap();
        // a comment after the pop
    }

    if let Some(x) = vec.last()
        && (vec.len() > 1 && *x > 10)
    //~^ manual_pop_if
    {
        vec.pop().unwrap();
    }

    let r = &vec;
    //~v manual_pop_if
    if let Some(x) = vec.last()
        && (r.len() > 1 && *x > 10)
    {
        vec.pop().unwrap();
    }
}

fn map_unwrap_or_pattern(mut vec: Vec<i32>) {
    //~v manual_pop_if
    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        let val = vec.pop().unwrap();
        println!("Popped: {}", val);
    }

    //~v manual_pop_if
    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        println!("Popped: {}", vec.pop().unwrap());
    }

    //~v manual_pop_if
    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        // a comment before the pop
        vec.pop().unwrap();
    }

    //~v manual_pop_if
    if vec.last().map(|x| *x > 2).unwrap_or(false) {
        vec.pop().unwrap();
        // a comment after the pop
    }

    //~v manual_pop_if
    if vec.last().map(|x| vec.len() > 1 && *x > 10).unwrap_or(false) {
        vec.pop().unwrap();
    }

    let r = &vec;
    //~v manual_pop_if
    if vec.last().map(|x| r.len() > 1 && *x > 10).unwrap_or(false) {
        vec.pop().unwrap();
    }
}

fn complex_collection(mut wrapper: Wrapper) {
    //~v manual_pop_if
    if wrapper.vec.last().is_some_and(|x| *x > 10) {
        wrapper.vec.pop().unwrap();
    }
}
