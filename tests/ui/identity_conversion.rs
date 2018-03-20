#![deny(identity_conversion)]

fn test_generic<T: Copy>(val: T) -> T {
    let _ = T::from(val);
    val.into()
}

fn test_generic2<T: Copy + Into<i32> + Into<U>, U: From<T>>(val: T) {
    // ok
    let _: i32 = val.into();
    let _: U = val.into();
    let _ = U::from(val);
}

fn test_questionmark() -> Result<(), ()> {
    {
        let _: i32 = 0i32.into();
        Ok(Ok(()))
    }??;
    Ok(())
}

fn test_iter<I: Iterator>(iter: &mut I) {
    iter.by_ref().for_each(drop)
} 

fn test_as_ref<S: ?Sized + AsRef<[u8]>>(s: &S) -> &[u8] {
    &s.as_ref()[1..]
} 

fn test_as_mut<S: ?Sized + AsMut<[u8]>>(s: &mut S) {
    &s.as_mut().sort();
}

fn main() {
    test_generic(10i32);
    test_generic2::<i32, i32>(10i32);
    test_questionmark().unwrap();
    test_iter(&mut [1, 2, 3].iter());
    test_as_ref(&b"hello"[..]);
    test_as_mut(&mut [4, 3, 2, 1]);

    let _: String = "foo".into();
    let _: String = From::from("foo");
    let _ = String::from("foo");
    #[allow(identity_conversion)]
    {
        let _: String = "foo".into();
        let _ = String::from("foo");
    }

    let _: String = "foo".to_string().into();
    let _: String = From::from("foo".to_string());
    let _ = String::from("foo".to_string());
}
