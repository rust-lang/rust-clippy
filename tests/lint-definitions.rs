#![feature(rustc_private)]

use clippy_lints::declared_lints::LINTS;
use clippy_lints::deprecated_lints::DEPRECATED;
use test_utils::IS_RUSTC_TEST_SUITE;

mod test_utils;

#[test]
fn no_nightly_versions_in_rust_repo() {
    if !IS_RUSTC_TEST_SUITE {
        return;
    }

    let mut failed = false;

    for lint in LINTS {
        if lint.version == "nightly" {
            let name = lint.name_lower();
            let location = lint.location.replace("#L", ":");
            println!(r#"{location}: {name} has #[clippy::version = "nightly"]"#);
            failed = true;
        }
    }

    for deprecation in DEPRECATED {
        if deprecation.version == "nightly" {
            let name = deprecation.name.strip_prefix("clippy::").unwrap();
            println!(r#"clippy_lints/src/deprecated_lints.rs: {name} has #[clippy::version = "nightly"]"#);
            failed = true;
        }
    }

    assert!(!failed);
}
