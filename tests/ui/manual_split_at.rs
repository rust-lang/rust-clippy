#![warn(clippy::manual_split_at)]
#![expect(clippy::useless_vec)]

fn main() {
    // one-line assignments
    let k = 3;

    let v = vec![1, 2, 3, 4, 5];
    let (left, right) = (&v[..k], &v[k..]);
    //~^ manual_split_at

    let slc: &[i32] = &[1, 2, 3, 4, 5];
    let (left, right) = (&slc[..k], &slc[k..]);
    //~^ manual_split_at

    let arr = [1, 2, 3, 4, 5];
    let (left, right) = (&arr[..k], &arr[k..]);
    //~^ manual_split_at

    let str_ = "hello";
    let (left, right) = (&str_[..k], &str_[k..]);
    //~^ manual_split_at

    let string_ = String::from("hello");
    let (left, right) = (&string_[..k], &string_[k..]);
    //~^ manual_split_at

    // field access as index
    struct Struct {
        field: usize,
    }
    let strct = Struct { field: 2 };

    let (left, right) = (&v[..strct.field], &v[strct.field..]);
    //~^ manual_split_at

    // literal as index
    let (left, right) = (&v[..2], &v[2..]);
    //~^ manual_split_at

    // order of slices is swapped, binders are swapped in the suggestion
    let (right, left) = (&v[k..], &v[..k]);
    //~^ manual_split_at

    let ([first, ..], right) = (&v[..k], &v[k..]) else {
        //~^ manual_split_at
        return;
    };

    // works when type is specified
    let (right, left): (&[i32], &[i32]) = (&v[k..], &v[..k]);
    //~^ manual_split_at

    // partially specified
    let (right, left): (&[i32], _) = (&v[k..], &v[..k]);
    //~^ manual_split_at

    // swapped with empty binders
    let (_, _) = (&v[k..], &v[..k]);
    //~^ manual_split_at

    // single binding, only the initializer is replaced
    let x = (&v[..k], &v[k..]);
    //~^ manual_split_at

    // different receivers - no lint
    let v1 = vec![1, 2, 3];
    let v2 = vec![1, 2, 3];
    let (_d1, _d2) = (&v1[..k], &v2[k..]);

    // index is not the same - no lint
    let (_d1, _d2) = (&v[..k], &v[k + 1..]);

    // possible side effects - no lint
    let (_d1, _d2) = (&v[..get_idx()], &v[get_idx()..]);

    // both ranges are OpFrom - no lint
    let (_d1, _d2) = (&arr[k..], &arr[k..]);

    // both ranges are OpTo - no lint
    let (_d1, _d2) = (&arr[..k], &arr[..k]);

    // swapped order but pattern is not a tuple - no lint
    let _y = (&arr[k..], &arr[..k]);

    // both ranges are inf - no lint
    let (_d1, _d2) = (&arr[..], &arr[..]);

    // reciever might have side effects - no lint
    let (_d1, _d2) = (&arr_func()[..k], &arr_func()[k..]);

    // two-line assignments

    //~v manual_split_at
    let left = &v[..k];
    let right = &v[k..];

    // swapped order works as well
    //~v manual_split_at
    let right = &v[k..];
    let left = &v[..k];

    //~v manual_split_at
    let left = &v[..strct.field];
    let right = &v[strct.field..];

    // mixed explicit type
    //~v manual_split_at
    let left: &[i32] = &v[..k];
    let right = &v[k..];

    //~v manual_split_at
    let left: &[i32] = &v[..k];
    let right: &[i32] = &v[k..];

    // different recievers - no lint
    let left: &[i32] = &v1[..k];
    let right: &[i32] = &v2[k..];

    // possible side effects - no lint
    let left = &arr_func()[..k];
    let right = &arr_func()[k..];

    let left = &v[..get_idx()];
    let right = &v[get_idx()..];
}

fn arr_func() -> [i32; 3] {
    [9, 1, 2]
}

fn get_idx() -> usize {
    2
}

#[clippy::msrv = "1.3"]
fn before_str_split_at_msrv() {
    let t = "hello";
    let k = 3;
    let (_, _) = (&t[..k], &t[k..]);

    let _ = &t[..k];
    let _ = &t[k..];
}

#[clippy::msrv = "1.4"]
fn after_str_split_at_msrv() {
    let t = "hello";
    let k = 3;

    let (_, _) = (&t[..k], &t[k..]);
    //~^ manual_split_at

    //~v manual_split_at
    let _a = &t[..k];
    let _b = &t[k..];
}
