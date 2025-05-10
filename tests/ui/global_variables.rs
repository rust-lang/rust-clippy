#![warn(clippy::global_variables)]

use std::sync::Mutex;
use std::sync::atomic::AtomicU32;

macro_rules! define_global_variable_with_macro {
    () => {
        static GLOBAL_VARIABLE_0: AtomicU32 = AtomicU32::new(2);
        //~^ global_variables

        static GLOBAL_VARIABLE_1: Mutex<u32> = Mutex::new(3);
        //~^ global_variables
    };
}

#[allow(clippy::missing_const_for_thread_local)]
fn main() {
    define_global_variable_with_macro!();

    static GLOBAL_VARIABLE_2: AtomicU32 = AtomicU32::new(2);
    //~^ global_variables

    static GLOBAL_VARIABLE_3: Mutex<u32> = Mutex::new(3);
    //~^ global_variables

    static GLOBAL_VARIABLE_4: Option<AtomicU32> = Some(AtomicU32::new(0));
    //~^ global_variables

    static NOT_GLOBAL_VARIABLE_0: u32 = 1;

    // Does not work yet: ZST fields should be ignored.
    // static NOT_GLOBAL_VARIABLE_1: Vec<AtomicU32> = Vec::new();

    // Does not work yet: Initializer with variant value that does not contain interior fields
    // should not be considered global variable.
    // static NOT_GLOBAL_VARIABLE_2: Option<AtomicU32> = None;

    // Thread-local variables are ignored.
    thread_local! {
        static THREAD_LOCAL_VARIABLE_0: u32 = 0;
        static THREAD_LOCAL_VARIABLE_1: AtomicU32 = AtomicU32::new(0);
        static THREAD_LOCAL_VARIABLE_2: u32 = const { 0 };
        static THREAD_LOCAL_VARIABLE_3: AtomicU32 = const { AtomicU32::new(0) };

        static THREAD_LOCAL_VARIABLE_4: () = {
            // Global variables inside a thread-local initializer are also considered.
            static GLOBAL_VARIABLE_IN_THREAD_LOCAL: AtomicU32 = AtomicU32::new(0);
            //~^ global_variables
        };
    }
}
