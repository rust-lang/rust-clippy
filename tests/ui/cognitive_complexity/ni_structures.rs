#![allow(clippy::all)]
#![warn(clippy::cognitive_complexity)]
#![allow(unused)]

fn main() {}

/*
    Tests for Nesting-Independent Structures
*/

#[clippy::cognitive_complexity = "0"]
fn func_calls() {
    let _ = Vec::new();
}

#[clippy::cognitive_complexity = "0"]
fn macro_calls() {
    println!("Hello!");
}

mod loop_change_statements {

    /* 
        ### We check that their most basic form scores 1 ### 
    */

    #[clippy::cognitive_complexity = "1"]
    fn b_reak() {
        loop {
            break;
        }
    }

    #[clippy::cognitive_complexity = "1"]
    fn c_ontinue() {
        loop {
            continue;
        }
    }

    #[clippy::cognitive_complexity = "2"]
    fn break_plus_ni_expr() -> bool {
        loop {
            break {
                true || false
            }
        }
    }

    #[clippy::cognitive_complexity = "3"]
    fn break_plus_nd_expr() -> bool {
        loop {
            break {
                if(true) {
                    println!("Hello");
                }
            }
        }
    }
}

mod binary_logic {

    /*
        ### The base cases should score 1 ###
    */

    #[clippy::cognitive_complexity = "0"]
    fn and(a: bool, b: bool) -> bool {
        a && b
    }

    #[clippy::cognitive_complexity = "0"]
    fn or(a: bool, b: bool) -> bool {
        a || b
    }

    #[clippy::cognitive_complexity = "0"]
    fn bit_and(a: bool, b: bool) -> bool {
        a & b
    }

    #[clippy::cognitive_complexity = "0"]
    fn bit_or(a: bool, b: bool) -> bool {
        a | b
    }

    #[clippy::cognitive_complexity = "0"]
    fn bit_xor(a: bool, b: bool) -> bool {
        a ^ b
    }

    /* 
        ### All of these should score 2 ### 
    */

    #[clippy::cognitive_complexity = "1"]
    fn change_one_1(a: bool, b: bool, c: bool) -> bool {
        a && b || c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_2(a: bool, b: bool, c: bool) -> bool {
        a && b & c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_3(a: bool, b: bool, c: bool) -> bool {
        a && b | c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_4(a: bool, b: bool, c: bool) -> bool {
        a && b ^ c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_5(a: bool, b: bool, c: bool) -> bool {
        a || b & c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_6(a: bool, b: bool, c: bool) -> bool {
        a || b | c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_7(a: bool, b: bool, c: bool) -> bool {
        a || b ^ c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_8(a: bool, b: bool, c: bool) -> bool {
        a & b | c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_9(a: bool, b: bool, c: bool) -> bool {
        a & b ^ c
    }

    #[clippy::cognitive_complexity = "1"]
    fn change_one_10(a: bool, b: bool, c: bool) -> bool {
        a | b ^ c
    }

    /* 
        ### These should score 2 as well ### 
    */
   
    #[clippy::cognitive_complexity = "1"]
    fn one_paren(a: bool, b: bool, c: bool) -> bool {
        a && (b || c)
    }

    #[clippy::cognitive_complexity = "1"]
    fn two_paren(a: bool, b: bool, c: bool) -> bool {
        a & ((b | c))
    }

    #[clippy::cognitive_complexity = "1"]
    fn one_unary(a: bool, b: bool, c: bool) -> bool {
        a ^ b && !c
    }

    #[clippy::cognitive_complexity = "1"]
    fn one_unary_one_paren(a: bool, b: bool, c: bool) -> bool {
        a || !(b & c)
    }

    mod complex_patterns {

        /*
            These patterns are made to test that the 
            binary logic scoring algorithm recurses correctly
        */

        #[clippy::cognitive_complexity = "3"]
        fn right_side_is_a_trap_1(a: bool, b: bool, c: bool, d: bool, e: bool) -> bool {
            // Here, if the algorithm recurses to the right instead of the left,
            // it should produce a lower score.
            (a && b) || ((c ^ d) || e)
        }

        #[clippy::cognitive_complexity = "3"]
        fn right_side_is_a_trap_2(a: bool, b: bool, c: bool, d: bool, e: bool) -> bool {
            // Same as (right_side_is_a_trap_1), this tries to check that
            // the algorithm's direction is correct
            (a && b) || (((c ^ d) || e) || c)
        }

        #[clippy::cognitive_complexity = "0"]
        fn super_long_chain(a: bool, b: bool, c: bool, d: bool, e: bool) -> bool {
            a && b && c && d && e
        }
    }
}
