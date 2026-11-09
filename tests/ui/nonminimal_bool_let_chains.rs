//@edition: 2024

#![warn(clippy::nonminimal_bool, clippy::overly_complex_bool_expr)]

fn if_let_chains(opt: Option<u8>, a: bool, b: bool) {
    if let Some(_) = opt
        && !(a && !b)
    //~^ nonminimal_bool
    {}

    if !(a && !b)
        //~^ nonminimal_bool
        && let Some(_) = opt
    {}

    if let Some(_) = opt
        && !(a && !b)
        //~^ nonminimal_bool
        && let Some(_) = opt
    {}

    if let Some(_) = opt
        && a
        && !(a && !b)
    //~^ nonminimal_bool
    {}
}

fn while_let_chains(mut opt: Option<u8>, a: bool, b: bool) {
    while let Some(_) = opt
        && !(a && !b)
    //~^ nonminimal_bool
    {
        opt = None;
    }

    while !(a && !b)
        //~^ nonminimal_bool
        && let Some(_) = opt
    {
        opt = None;
    }
}

fn bound_variables(opt: Option<(bool, bool)>) {
    if let Some((a, b)) = opt
        && !(a && !b)
    //~^ nonminimal_bool
    {}
}

fn overly_complex_let_chain(opt: Option<u8>, a: bool, b: bool, c: bool) {
    if let Some(_) = opt
        && ((a || b) && (a || b || c))
    //~^ overly_complex_bool_expr
    {}
}

fn no_extra_parentheses(opt: Option<u8>, a: bool, b: bool) {
    if let Some(_) = opt
        && !(a || !b)
    //~^ nonminimal_bool
    {}

    if !(a && !b) {}
    //~^ nonminimal_bool
}

fn main() {}
