#![warn(clippy::uninlined_format_args)]
#![allow(clippy::useless_format)]

fn main() {
    format!("The answer is: {}", "42");
    //~^ uninlined_format_args
    format!("The answer is: {}", '4');
    //~^ uninlined_format_args

    let x = 1;
    format!("The answer is: {} and x is {}", "42", x);
    //~^ uninlined_format_args
    format!("The answer is: {} and x is {}", '4', x);
    //~^ uninlined_format_args

    let n_refs = 10;
    let receiver_snippet = "receiver_snippet";
    format!("{:&>n_refs$}{receiver_snippet}", "");

    format!("env: {}", env!("USER"));

    format!("{}", "{}");
    //~^ uninlined_format_args
}
