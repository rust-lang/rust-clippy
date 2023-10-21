#![deny(clippy::redundant_clone)]

use std::hint::black_box;

fn main() {
    {
        let x = String::new();
        let _ = x.clone();
    }
    {
        let x = String::new();
        let _x = x.clone();
    }
    {
        let x = String::new();
        let _ = black_box(x.clone());
    }
    {
        let x = String::new();
        let _ = black_box(&mut x.clone());
    }
    {
        let x = String::new();
        let mut y = x.clone();
        y.push_str("xx");
        println!("{y} {x}");
    }
    {
        let x = String::new();
        let _ = black_box(x.clone());
        println!("{x}");
    }
    {
        let x = String::new();
        let _ = black_box(&mut x.clone());
        println!("{x}");
    }
    {
        let x = String::new();
        let y = String::new();
        let z = if black_box(true) { &x } else { &y };

        black_box(x.clone());
        black_box(y.clone());
        black_box(z.clone());

        println!("{z}");
    }
}
