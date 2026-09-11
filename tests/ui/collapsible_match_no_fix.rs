//@no-rustfix
#![warn(clippy::collapsible_match)]
#![allow(clippy::single_match)]

// https://github.com/rust-lang/rust-clippy/issues/17427
// A labeled block whose label AND block both originate from a macro expansion
// must not attempt a suggestion, since span arithmetic across the macro
// boundary previously caused an ICE.
macro_rules! labelled {
    ($label:tt, $($stmts:stmt);*) => {
        $label: { $($stmts);* }
    };
}

fn issue17427_macro_label_and_block() {
    let x: Result<Option<u8>, ()> = Ok(Some(1));
    match x {
        Ok(Some(_c)) => labelled!(
            'label,
            if true { println!("hi") }
            //~^ collapsible_match
        ),
        _ => (),
    }
}

fn main() {}
