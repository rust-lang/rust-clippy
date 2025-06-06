#![deny(clippy::direct_recursion)]
#![deny(clippy::allow_attributes_without_reason)]

// Basic Cases //

#[allow(unconditional_recursion, reason = "We're not testing for that lint")]
fn i_call_myself_always() {
    i_call_myself_always();
    //~^ direct_recursion
}

fn i_call_myself_conditionally(do_i: bool) {
    if do_i {
        i_call_myself_conditionally(false);
        //~^ direct_recursion
    }
}

// Basic Counterexamples //

fn i_get_called_by_others() {}

fn i_call_something_else() {
    i_get_called_by_others();
}

// Elaborate Cases //

// Here we check that we're allowed to bless specific recursive calls.
// A fine-grained control of where to allow recursion is desirable.
// This is a test of such a feature.
fn i_call_myself_in_a_bounded_way(bound: u8) {
    if bound > 0 {
        #[expect(
            clippy::direct_recursion,
            reason = "Author has audited this function and determined that its recursive call is fine."
        )]
        i_call_myself_in_a_bounded_way(bound - 1);
    }
}

// Here we check that blessing a specific recursive call doesn't
// let other recursive calls go through.
fn i_have_one_blessing_but_two_calls(bound: u8) {
    if bound > 25 {
        #[expect(
            clippy::direct_recursion,
            reason = "Author has audited this function and determined that its recursive call is fine."
        )]
        i_have_one_blessing_but_two_calls(bound - 1);
    } else if bound > 0 {
        // "WIP: we still need to audit this part of the function"
        i_have_one_blessing_but_two_calls(bound - 2)
        //~^ direct_recursion
    }
}
