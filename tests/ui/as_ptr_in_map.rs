#![warn(clippy::as_ptr_in_map)]

fn main() {
    let v1_res = Ok(vec![1]);
    let _v1_ptr: Result<_, ()> = v1_res.map(|v1| v1.as_ptr());
    //~^ as_ptr_in_map

    let v2_opt = Some((vec![2], 2));
    let _v2_ptr = v2_opt.map(|(v2, _x)| {
        v2.as_ptr();
        2
    });
    // this is fine

    let v3_opt = Some((vec![3], 3));
    let _v3_ptr = v3_opt.map(|(v3, x)| {
        //~^ as_ptr_in_map
        let _a = x + 2;
        let p = v3.as_ptr();
        let _b = 6;
        p
    });

    let v4_res = Ok(vec![4]);
    let _v4_ptr: Result<_, &()> = v4_res.as_ref().map(|v4| v4.as_ptr());
    // this is fine

    let v5_opt = Some(vec![5]);
    let _v5_ptr = v5_opt.map(|v5| std::vec::Vec::as_ptr(&v5));
    //~^ as_ptr_in_map

    let v6_res = Ok(vec![6]);
    let v6_2_ptr = [6];
    let _v6_ptr: Result<_, ()> = v6_res.map(|_v6| v6_2_ptr.as_ptr());
    // this is fine
}
