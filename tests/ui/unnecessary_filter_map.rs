fn main() {
    let _ = (0..4).filter_map(|x| if x > 1 { Some(x) } else { None });
    let _ = (0..4).filter_map(|x| {
        if x > 1 {
            return Some(x);
        };
        None
    });
    let _ = (0..4).filter_map(|x| match x {
        0 | 1 => None,
        _ => Some(x),
    });

    let _ = (0..4).filter_map(|x| Some(x + 1));

    let _ = (0..4).filter_map(i32::checked_abs);

    struct S(u32);
    impl S {
        fn mutate(&mut self) {}
    }

    // Mutating
    {
        let _: Vec<_> = vec![S(0), S(1), S(2)]
            .into_iter()
            .filter_map(|mut x| {
                x.mutate();
                if x.0 % 2 == 0 {
                    Some(x)
                } else {
                    None
                }
            })
            .collect();
    }

    // Moving
    {
        let mut v = vec![];
        let _: Vec<_> = vec![S(0), S(1), S(2)]
            .into_iter()
            .filter_map(|x| {
                if x.0 % 2 == 0 {
                    Some(x)
                } else {
                    v.push(x);
                    None
                }
            })
            .collect();
    }

    enum E {
        A,
        B(S),
    }

    // Mutating with pattern
    {
        let _: Vec<_> = vec![E::A, E::B(S(0))]
            .into_iter()
            .filter_map(|mut x| {
                if let E::B(ref mut y) = x {
                    y.mutate();
                    None
                } else {
                    Some(x)
                }
            })
            .collect();
    }

    // Moving with pattern
    {
        let mut v = vec![];
        let _: Vec<_> = vec![E::A, E::B(S(0))]
            .into_iter()
            .filter_map(|x| {
                if let E::B(y) = x {
                    if y.0 % 2 == 0 {
                        v.push(y);
                    }
                    return None;
                }
                Some(x)
            })
            .collect();
    }
}
