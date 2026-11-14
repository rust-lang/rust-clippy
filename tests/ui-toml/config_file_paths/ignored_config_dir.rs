//@check-pass
//@rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/config_file_paths/config
#![warn(clippy::disallowed_names)]

fn main() {
    let selected = 0;
}
