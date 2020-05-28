// aux-build:doc_unsafe_macros.rs

#[macro_use]
extern crate doc_unsafe_macros;

/// This is not sufficiently documented
pub unsafe fn destroy_the_planet() {
    unimplemented!();
}

/// This one is
///
/// # Safety
///
/// This function shouldn't be called unless the horsemen are ready
pub unsafe fn apocalypse(universe: &mut ()) {
    unimplemented!();
}

/// This is a private function, so docs aren't necessary
unsafe fn you_dont_see_me() {
    unimplemented!();
}

mod private_mod {
    pub unsafe fn only_crate_wide_accessible() {
        unimplemented!();
    }

    pub unsafe fn republished() {
        unimplemented!();
    }
}

pub use private_mod::republished;

pub trait UnsafeTrait {
    unsafe fn woefully_underdocumented(self);

    /// # Safety
    unsafe fn at_least_somewhat_documented(self);
}

pub struct Struct;

impl UnsafeTrait for Struct {
    unsafe fn woefully_underdocumented(self) {
        // all is well
    }

    unsafe fn at_least_somewhat_documented(self) {
        // all is still well
    }
}

impl Struct {
    pub unsafe fn more_undocumented_unsafe() -> Self {
        unimplemented!();
    }

    /// # Safety
    pub unsafe fn somewhat_documented(&self) {
        unimplemented!();
    }

    unsafe fn private(&self) {
        unimplemented!();
    }
}

macro_rules! very_unsafe {
    () => {
        pub unsafe fn whee() {
            unimplemented!()
        }

        /// # Safety
        ///
        /// Please keep the seat belt fastened
        pub unsafe fn drive() {
            whee()
        }
    };
}

very_unsafe!();

// we don't lint code from external macros
undocd_unsafe!();

/** This safety section is now recognized.

   # Safety

   No risk, no fun.
*/
pub unsafe fn strange_doc_comments() {
    unimplemented!();
}

/// This is not a # Safety section.
pub unsafe fn ceci_n_est_pas_un_titre() {
    unimplemented!();
}

fn main() {
    unsafe {
        you_dont_see_me();
        destroy_the_planet();
        let mut universe = ();
        apocalypse(&mut universe);
        private_mod::only_crate_wide_accessible();
        drive();
        strange_doc_comments();
        ceci_n_est_pas_un_titre();
    }
}
