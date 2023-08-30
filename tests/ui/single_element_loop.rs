// Tests from for_loop.rs that don't have suggestions

#![allow(clippy::single_range_in_vec_init)]

#[warn(clippy::single_element_loop)]
fn main() {
    let item1 = 2;
    for item in &[item1] {
    //~^ ERROR: for loop over a single element
    //~| NOTE: `-D clippy::single-element-loop` implied by `-D warnings`
        dbg!(item);
    }

    for item in [item1].iter() {
    //~^ ERROR: for loop over a single element
        dbg!(item);
    }

    for item in &[0..5] {
    //~^ ERROR: for loop over a single element
        dbg!(item);
    }

    for item in [0..5].iter_mut() {
    //~^ ERROR: for loop over a single element
        dbg!(item);
    }

    for item in [0..5] {
    //~^ ERROR: for loop over a single element
        dbg!(item);
    }

    for item in [0..5].into_iter() {
    //~^ ERROR: for loop over a single element
        dbg!(item);
    }

    // should not lint (issue #10018)
    for e in [42] {
        if e > 0 {
            continue;
        }
    }

    // should not lint (issue #10018)
    for e in [42] {
        if e > 0 {
            break;
        }
    }

    // should lint (issue #10018)
    for _ in [42] {
    //~^ ERROR: for loop over a single element
        let _f = |n: u32| {
            for i in 0..n {
                if i > 10 {
                    dbg!(i);
                    break;
                }
            }
        };
    }
}
