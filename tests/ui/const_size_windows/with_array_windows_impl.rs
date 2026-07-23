#![warn(clippy::const_size_windows)]

trait DefinesArrayWindows {
    fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
        ['🪟'; LENGTH]
    }
}

#[derive(Debug)]
struct ArrayWindowsStub;

impl DefinesArrayWindows for Vec<ArrayWindowsStub> {}

mod array_methods_mod {
    trait InnerDefinesArrayWindows {
        fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
            ['🪟'; LENGTH]
        }
    }
    #[derive(Debug)]
    pub struct ArrayWindowsStub;
    impl InnerDefinesArrayWindows for Vec<ArrayWindowsStub> {}
}

use array_methods_mod::ArrayWindowsStub as ModArrayWindowsStub;

fn array_windows_implemented_by_public_trait(vec: Vec<ArrayWindowsStub>) {
    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_in_mod(vec: Vec<ModArrayWindowsStub>) {
    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[ModArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_nested_trait() {
    #[derive(Debug)]
    struct Stub;
    trait DefinesArrayWindows {
        fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
            ['🪟'; LENGTH]
        }
    }

    impl DefinesArrayWindows for Vec<Stub> {}

    #[allow(clippy::useless_vec)]
    let vec = vec![Stub, Stub, Stub];
    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[Stub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_public_trait_with_macro() {
    #[expect(clippy::useless_vec)]
    for pair in vec![ArrayWindowsStub, ArrayWindowsStub, ArrayWindowsStub].windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_borrowed(#[expect(clippy::ptr_arg)] vec: &Vec<ArrayWindowsStub>) {
    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_dereferenced(#[allow(clippy::ptr_arg)] vec: &Vec<ArrayWindowsStub>) {
    for pair in (*vec).windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_with_ref() {
    #[derive(Debug)]
    struct Stub;
    impl DefinesArrayWindows for &Vec<Stub> {}

    let vec = vec![Stub, Stub, Stub];
    #[allow(clippy::needless_borrow)]
    for pair in (&vec).windows(2) {
        //~^ const_size_windows
        let pair: &[Stub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_with_deref(
    derefs_into_vec_stub: impl std::ops::Deref<Target = Vec<ArrayWindowsStub>>,
) {
    for pair in (*derefs_into_vec_stub).windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_by_trait_with_deref_coercion(
    derefs_into_vec_stub: impl std::ops::Deref<Target = Vec<ArrayWindowsStub>>,
) {
    for pair in derefs_into_vec_stub.windows(2) {
        //~^ const_size_windows
        let pair: &[ArrayWindowsStub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_not_implemented_by_trait(vec: Vec<String>) {
    #[derive(Debug)]
    struct Stub;
    impl DefinesArrayWindows for Vec<Stub> {}

    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[String] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}

fn array_windows_implemented_with_double_borrowed_vec_macro() {
    macro_rules! double_borrowed_vec {
        ($($x:expr),* $(,)?) => {
            &&vec![$($x),*]
        };
    }

    #[derive(Debug)]
    struct Stub;
    trait DefinesArrayWindows {
        fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
            ['🪟'; LENGTH]
        }
    }

    impl DefinesArrayWindows for &&Vec<Stub> {}

    for pair in double_borrowed_vec![Stub, Stub, Stub].windows(2) {
        //~^ const_size_windows
        let pair: &[Stub] = pair;
        println!("{:#?} {:#?}", pair[0], pair[1]);
    }
}
