#![warn(clippy::disallowed_methods)]

fn main() {
    renamed_dependency::forbidden_by_alias();
    renamed_dependency::forbidden_by_package_name();
    other_dependency::allowed();
}
