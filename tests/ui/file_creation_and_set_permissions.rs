#![warn(clippy::file_creation_and_set_permissions)]
#![feature(set_permissions_nofollow)]
#![feature(file_buffered)]

use std::fs::{File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn main() {
    // don't lint here
    std::fs::set_permissions("bar_two", Permissions::from_mode(0o700));
    // don't lint here
    std::fs::set_permissions_nofollow("bar_two", Permissions::from_mode(0o700));

    let path_three = Path::new("foo_three");
    // don't lint here
    std::fs::set_permissions(path_three, Permissions::from_mode(0o700));
    // don't lint here
    std::fs::set_permissions_nofollow(path_three, Permissions::from_mode(0o700));
}

fn create_dir_followed_by_set_perms() {
    let path = Path::new("foo");
    std::fs::create_dir(path);
    // lint here
    std::fs::set_permissions(path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    std::fs::set_permissions_nofollow(path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    let path_two = Path::new("foo_two");
    let _ = std::fs::create_dir(path_two);
    // lint here
    let _ = std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    std::fs::create_dir("bar");
    // lint here
    std::fs::set_permissions("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    // variable shadowing path_two that is not used by create_dir
    // should not trigger a lint
    let path_two = Path::new("foo_four");
    // don't lint here
    std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
    // don't lint here
    std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
}

fn create_dir_all_followed_by_set_perms() {
    let path = Path::new("foo");
    std::fs::create_dir_all(path);
    // lint here
    std::fs::set_permissions(path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow(path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    let path_two = Path::new("foo_two");
    let _ = std::fs::create_dir_all(path_two);
    // lint here
    let _ = std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    std::fs::create_dir_all("bar");
    // lint here
    std::fs::set_permissions("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions

    // variable shadowing path_two that is not used by create_dir
    // should not trigger a lint
    let path_two = Path::new("foo_four");
    // don't lint here
    std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
    // don't lint here
    std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
}

fn create_dir_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
    std::fs::create_dir(&path);
    // lint here
    std::fs::set_permissions(&path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
}

fn create_dir_all_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
    std::fs::create_dir_all(&path);
    // lint here
    std::fs::set_permissions(&path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
}

fn file_create_followed_by_set_perms() {
    File::create("bar").unwrap();
    // lint here
    std::fs::set_permissions("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
}

fn file_create_new_followed_by_set_perms() {
    File::create_new("bar").unwrap();
    // lint here
    std::fs::set_permissions("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
}

fn file_create_buffered_followed_by_set_perms() {
    File::create_buffered("bar").unwrap();
    // lint here
    std::fs::set_permissions("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
    // lint here
    std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
    //~^ file_creation_and_set_permissions
}
