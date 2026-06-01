#![warn(clippy::error_format_without_sources)]

//@aux-build:proc_macros.rs
extern crate proc_macros;

use std::error::Error;
use std::fmt::Display;
use std::io;

#[derive(Debug)]
pub enum ConfigReadError {
    ConfigFileOpenFailed(io::Error),
}

impl Display for ConfigReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigReadError::ConfigFileOpenFailed(_) => {
                write!(f, "Failed to open config file")
            },
        }
    }
}
impl Error for ConfigReadError {
    // The lint only triggers when source is implemented
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigReadError::ConfigFileOpenFailed(source) => Some(source),
        }
    }
}

fn open_config() -> Result<(), ConfigReadError> {
    Err(ConfigReadError::ConfigFileOpenFailed(io::Error::other(
        "The code didn't even try to open a file",
    )))
}

// Also trigger lint on macro expansions
macro_rules! my_error_macro {
    ($error_name:ident) => {
        #[derive(Debug)]
        enum $error_name {
            ReadConfigFailed(ConfigReadError),
        }

        impl Display for $error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::ReadConfigFailed(source_error) => {
                        write!(f, "Failed to read config file: {}", source_error)
                    },
                }
            }
        }

        impl Error for $error_name {
            // Ensure the lint will trigger
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                None
            }
        }
    };
}
my_error_macro!(MacroTestError);
//~^ error_format_without_sources

fn read_config_macro() -> Result<(), MacroTestError> {
    open_config().map_err(MacroTestError::ReadConfigFailed)
}

// Might as well test this as well
fn test() {
    proc_macros::external! { //~ error_format_without_sources
        if let Err(err) = open_config() {
            err.to_string();
        }
    }

    proc_macros::external! { //~ error_format_without_sources
        if let Err(err) = open_config() {
            println!("Failed to read config: {}", err);
        }
    }
}

// Confirm allow works
#[allow(clippy::error_format_without_sources)]
fn test_allow() {
    if let Err(err) = open_config() {
        eprintln!("Failed to read config: {err}");
    }
}
fn test_allow2() {
    #[allow(clippy::error_format_without_sources)]
    if let Err(err) = open_config() {
        eprintln!("Failed to read config: {err}");
    }

    proc_macros::external! {
        #[allow(clippy::error_format_without_sources)]
        if let Err(err) = open_config() {
            println!("Failed to read config: {}", err);
        }
    }
}

// Also confirm this weird case doesn't trigger a FP diagnostic
#[derive(Debug)]
pub struct ErrorWithOwnToStringImpl;

impl ErrorWithOwnToStringImpl {
    #[allow(clippy::inherent_to_string_shadow_display)]
    fn to_string(&self) -> String {
        String::new()
    }
}

impl Display for ErrorWithOwnToStringImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error type that has a to_string method")
    }
}

impl Error for ErrorWithOwnToStringImpl {}

fn main() {
    let result = open_config();
    if let Err(err) = result {
        println!("Failed to read config: {}", err);
        //~^ error_format_without_sources
        println!("Failed to read config: {:}", err);
        //~^ error_format_without_sources
        println!("Failed to read config: {:#}", err);
        //~^ error_format_without_sources
        println!("Failed to read config: {:?}", err);
        //~^ error_format_without_sources
        println!("Failed to read config: {:#?}", err);
        //~^ error_format_without_sources
        println!("Failed to read config: {err}");
        //~^ error_format_without_sources
        eprintln!("Failed to read config: {}", err);
        //~^ error_format_without_sources
        eprintln!("Failed to read config: {err}");
        //~^ error_format_without_sources

        let err_description_without_sources = err.to_string();
        //~^ error_format_without_sources

        // Some control examples that shouldn't raise lints
        fn to_string(err: &ConfigReadError) -> String {
            String::from("foo")
        }
        // Not the to_string we're interested in
        to_string(&err);
        // Not the to_string method at all
        err.source();
    }

    if let Err(err) = read_config_macro() {
        err.to_string();
        //~^ error_format_without_sources
        eprintln!("{}", err);
        //~^ error_format_without_sources
    }

    // Not an Error type
    let some_other_thing = "Hello, world!";
    println!("{}", some_other_thing);
    println!("{some_other_thing}");
    some_other_thing.to_string();

    // Not the to_string method from the ToString trait
    let error_with_to_string = ErrorWithOwnToStringImpl;
    ErrorWithOwnToStringImpl.to_string();
    error_with_to_string.to_string();
}

#[derive(Debug)]
pub struct ErrorWithoutSource {}

impl Display for ErrorWithoutSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown error, for testing purposes")
    }
}
impl std::error::Error for ErrorWithoutSource {}

fn test_error_without_source() {
    let base_error = ErrorWithoutSource {};
    // None of these should trigger the lint
    println!("{}", base_error);
    base_error.to_string();
}
