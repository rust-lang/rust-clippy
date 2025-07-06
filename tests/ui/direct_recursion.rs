#![deny(clippy::direct_recursion)]
#![feature(stmt_expr_attributes)]

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

// Case 1: Blessing //
// Here we check that we're allowed to bless specific recursive calls.
// A fine-grained control of where to allow recursion is desirable.
// This is a test of such a feature.
fn i_call_myself_in_a_bounded_way(bound: u8) {
    if bound > 0 {
        #[expect(clippy::direct_recursion)]
        i_call_myself_in_a_bounded_way(bound - 1);
    }
}

// Case 2: Blessing is Bounded //
// Here we check that blessing a specific recursive call doesn't
// let other recursive calls go through.
fn i_have_one_blessing_but_two_calls(bound: u8) {
    if bound > 25 {
        #[expect(clippy::direct_recursion)]
        i_have_one_blessing_but_two_calls(bound - 1);
    } else if bound > 0 {
        // "WIP: we still need to audit this part of the function"
        i_have_one_blessing_but_two_calls(bound - 2)
        //~^ direct_recursion
    }
}

// Case 3: Blessing is Recursive //
// Here we check that blessing a specific expression will
// bless everything inside of that expression as well.
fn ackermann(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        #[expect(clippy::direct_recursion)]
        ackermann(m - 1, 1)
    } else {
        #[expect(clippy::direct_recursion)]
        ackermann(m, ackermann(m + 1, n))
    }
}

// Case 4: Linting is Recursive //
// Here we check that linting a specific expression will
// not block other expressions inside of it from being linted.
fn ackermann_2_electric_recursion(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        #[expect(clippy::direct_recursion)]
        ackermann_2_electric_recursion(m - 1, 1)
    } else {
        ackermann_2_electric_recursion(
            //~^ direct_recursion
            m,
            ackermann_2_electric_recursion(m + 1, n),
            //~^ direct_recursion
        )
    }
}

// Case 5: Nesting Functions //
// Here we check that nested functions calling their parents are still
// linted
fn grand_parent() {
    fn parent() {
        fn child() {
            parent();
            //~^ direct_recursion
            grand_parent();
            //~^ direct_recursion
        }
        grand_parent();
        //~^ direct_recursion
    }
}

// Case 6: Binding of path to a Fn pointer //
// Here we check that we are able to detect bindings of function names
// as edges for the function call graph.
fn fibo(a: u32) -> u32 {
    if a < 2 { a } else { (a - 2..a).map(fibo).sum() }
    //~^ direct_recursion
}

// Case 7: Linting on Associated Function Implementations //
// Here we check that different implementations of the same trait don't go
// linting calls to functions of implementations that are not their own.
trait RecSum {
    fn rec_sum(n: u32) -> u32;
}

struct Summer;

// Recursive Call: should be linted
impl RecSum for Summer {
    fn rec_sum(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            // Notice how this is a recursive call, whereas the next one isn't
            n + Self::rec_sum(n - 1)
            //~^ direct_recursion
        }
    }
}

struct Winter;

// Not a Recursive Call: should be ignored.
impl RecSum for Winter {
    fn rec_sum(n: u32) -> u32 {
        // This should NOT trigger the lint, because even though it's calling the same
        // function (or "the same symbol"), it's not recursively calling its own implementation.
        if n == 0 { 0 } else { n + Summer::rec_sum(n - 1) }
    }
}

// Case 8: Linting on Default Trait Method Implementations //
// Here we check that recursion in trait methods is also captured by the lint
trait MyTrait {
    fn myfun(&self, num: i32) {
        if num > 0 {
            self.myfun(num - 1);
            //~^ direct_recursion
        }
    }
}

// Case 9: Linting on Trait Method Implementations //

struct T(u32);

trait W {
    fn f(&self);
}

impl W for T {
    fn f(&self) {
        if self.0 > 0 {
            self.f()
            //~^ direct_recursion
        }
    }
}
