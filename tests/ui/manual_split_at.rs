#![warn(clippy::manual_split_at)]
#![expect(clippy::useless_vec)]

fn main() {
    let k = 3;

    let v = vec![1, 2, 3, 4, 5];
    let (left, right) = (&v[..k], &v[k..]);
    //~^ manual_split_at

    let v: &[i32] = &[1, 2, 3, 4, 5];
    let (left, right) = (&v[..k], &v[k..]);
    //~^ manual_split_at

    let a = [1, 2, 3, 4, 5];
    let (left, right) = (&a[..k], &a[k..]);
    //~^ manual_split_at

    let s = "hello";
    let (left, right) = (&s[..k], &s[k..]);
    //~^ manual_split_at

    let s = String::from("hello");
    let (left, right) = (&s[..k], &s[k..]);
    //~^ manual_split_at

    // field access as index
    struct Bar {
        bar: usize,
    }
    let point = Bar { bar: 2 };
    let v = vec![1, 2, 3];
    let (left, right) = (&v[..point.bar], &v[point.bar..]);
    //~^ manual_split_at

    // literal as index
    let v = vec![1, 2, 3];
    let (left, right) = (&v[..2], &v[2..]);
    //~^ manual_split_at

    // order of slices is swapped, binders are swapped in the suggestion
    let v = vec![1, 2, 3];
    let (right, left) = (&v[k..], &v[..k]);
    //~^ manual_split_at

    // swapped with empty binders
    let (_, _) = (&a[k..], &a[..k]);
    //~^ manual_split_at

    // single binding, only the initializer is replaced
    let x = (&a[..k], &a[k..]);
    //~^ manual_split_at

    // different receivers - no lint
    let v1 = vec![1, 2, 3];
    let v2 = vec![1, 2, 3];
    let (_d1, _d2) = (&v1[..k], &v2[k..]);

    // index is not the same - no lint
    let v = vec![1, 2, 3];
    let (_d1, _d2) = (&v[..k], &v[k + 1..]);

    // possible side effects - no lint
    let v = vec![1, 2, 3];
    let (_d1, _d2) = (&v[..get_idx()], &v[get_idx()..]);

    // both ranges are OpFrom - no lint
    let (_d1, _d2) = (&a[k..], &a[k..]);

    // both ranges are OpTo - no lint
    let (_d1, _d2) = (&a[k..], &a[k..]);

    // swapped order but pattern is not a tuple - no lint
    let _y = (&a[k..], &a[..k]);

    // both ranges are inf - no lint
    let (_d1, _d2) = (&a[..], &a[..]);

    // reciever might have side effects - no lint
    let (_d1, _d2) = (&some_arr()[..k], &some_arr()[k..]);
}

fn some_arr() -> [i32; 3] {
    [9, 1, 2]
}

fn get_idx() -> usize {
    2
}

#[clippy::msrv = "1.3"]
fn _msrv_1_3_str() {
    let t = "hello";
    let k = 3;
    let (_, _) = (&t[..k], &t[k..]);
}

#[clippy::msrv = "1.4"]
fn msrv_1_4_str() {
    let t = "hello";
    let k = 3;
    let (_, _) = (&t[..k], &t[k..]);
    //~^ manual_split_at
}
