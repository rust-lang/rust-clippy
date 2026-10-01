//@revisions: cargo config dotted plain priority nearest
//@[cargo] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/cargo
//@[config] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/config
//@[dotted] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/dotted
//@[plain] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/plain
//@[priority] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/priority
//@[nearest] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/nearest/child
//@[dotted] error-in-other-file: using config file
//@[dotted] error-in-other-file: using config file
//@[dotted] error-in-other-file: using config file
//@[plain] error-in-other-file: using config file
//@[plain] error-in-other-file: using config file
//@[priority] error-in-other-file: using config file
#![warn(clippy::disallowed_names)]

fn main() {
    let selected = 0;
    //~^ disallowed_names
    let ignored = 0;
}
