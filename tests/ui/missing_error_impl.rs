#![warn(clippy::missing_error_impl)]

struct InternalError;

pub(crate) struct CrateInternalError;

pub struct PublicError;
//~^ missing_error_impl

pub struct NotAnErrorType;

#[derive(Debug)]
pub struct GenericError<T>(T);

impl<T> core::fmt::Display for GenericError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
    }
}

impl<T> core::error::Error for GenericError<T> where T: core::fmt::Display + core::fmt::Debug {}
