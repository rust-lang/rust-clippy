#![feature(rustc_private)]

use clippy_lints::declared_lints::LINTS;
use clippy_lints::deprecated_lints::DEPRECATED;
use test_utils::IS_RUSTC_TEST_SUITE;

mod test_utils;

#[test]
fn no_dev_versions_in_rust_repo() {
    if !IS_RUSTC_TEST_SUITE {
        return;
    }

    let mut failed = false;

    for lint in LINTS {
        if lint.version == "dev" {
            let name = lint.name_lower();
            let location = lint.location.replace("#L", ":");
            println!(r#"{location}: {name} has #[clippy::version = "dev"]"#);
            failed = true;
        }
    }

    for deprecation in DEPRECATED {
        if deprecation.version == "dev" {
            let name = deprecation.name.strip_prefix("clippy::").unwrap();
            println!(r#"clippy_lints/src/deprecated_lints.rs: {name} has #[clippy::version = "dev"]"#);
            failed = true;
        }
    }

    assert!(!failed);
}
