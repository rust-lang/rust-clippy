// run-rustfix
#![warn(clippy::ref_mut_iter_method_chain)]

macro_rules! m {
    ($i:ident) => {
        $i
    };
    (&mut $i:ident) => {
        &mut $i
    };
    (($i:expr).$m:ident($arg:expr)) => {
        ($i).$m($arg)
    };
}

fn main() {
    let mut iter = [0, 1, 2].iter();
    let _ = (&mut iter).find(|&&x| x == 1);
    let _ = (&mut m!(iter)).find(|&&x| x == 1);

    // Don't lint. `&mut` comes from macro expansion.
    let _ = m!(&mut iter).find(|&&x| x == 1);

    // Don't lint. Method call from expansion
    let _ = m!((&mut iter).find(|&&x| x == 1));

    // Don't lint. No method chain.
    for &x in &mut iter {
        print!("{}", x)
    }

    let iter = &mut iter;
    (&mut *iter).find(|&&x| x == 1);
}
