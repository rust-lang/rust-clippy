#![warn(
    clippy::create_dir_and_set_permissions,
    clippy::create_regular_file_and_set_permissions,
    clippy::create_symlink_and_set_permissions
)]
#![feature(file_buffered)]
#![feature(set_permissions_nofollow)]
mod main {
    use std::fs::{File, Permissions};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    // ===============================================================================
    // Should not lint on `set_permissions*` if no prior file creation on that
    // path occurred
    fn no_prior_file_creation_calls() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions("bar_two", Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow("bar_two", Permissions::from_mode(0o700));

        let path_three = Path::new("foo_three");
        std::fs::set_permissions(path_three, Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow(path_three, Permissions::from_mode(0o700));
    }
    // ===============================================================================

    // ===============================================================================
    // The following tests linting `set_permissions*` against create_dir* and symlink*
    fn create_dir_followed_by_set_perms() {
        use std::os::unix::fs::PermissionsExt;
        let path = Path::new("foo");
        std::fs::create_dir(path);

        std::fs::set_permissions(path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let _ = std::fs::create_dir(path_two);
        let _ = std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        std::fs::create_dir("bar");
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
    }

    fn create_dir_all_followed_by_set_perms() {
        use std::os::unix::fs::PermissionsExt;

        let path = Path::new("foo");
        std::fs::create_dir_all(path);
        std::fs::set_permissions(path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir_all`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let _ = std::fs::create_dir_all(path_two);
        let _ = std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        std::fs::create_dir_all("bar");
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir_all`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
    }

    fn symlink_followed_by_set_perms() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let path = Path::new("foo");
        let link = Path::new("baz");
        symlink(path, link);
        std::fs::set_permissions(path, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(path, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir_all`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let link_two = Path::new("baz_two");
        let _ = symlink(path_two, link_two);
        let _ = std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions

        symlink("bar", "symlink_foo");
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir_all`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow(path_two, Permissions::from_mode(0o700));
    }

    // #[cfg(target_os = "windows")]
    // fn symlink_dir_followed_by_set_perms() {
    //     #![feature(windows_permissions_ext)]
    //     use std::os::windows::fs::{PermissionsExt, symlink_dir};

    //     let path = Path::new("foo");
    //     let link = Path::new("baz");
    //     symlink_dir(path, link);

    //     std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     // The `set_permissions*` should lint against the `create_dir_all`
    //     // below here and even during assignment to a variable.
    //     let path_two = Path::new("foo_two");
    //     let link_two = Path::new("baz_two");
    //     let _ = symlink_dir(path_two, link_two);
    //     let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     symlink_dir("bar", "symlink_foo");
    //     std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     // variable shadowing path_two that is not used by `create_dir_all`
    //     // should not trigger a lint
    //     let path_two = Path::new("foo_four");
    //     std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
    //     std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    // }

    // #[cfg(target_os = "wasi")]
    // fn symlink_path_followed_by_set_perms() {
    //     use std::os::wasi::fs::symlink_path;

    //     let path = Path::new("foo");
    //     let link = Path::new("baz");
    //     let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    //     permissions.set_readonly(true);

    //     symlink_path(path, link);
    //     std::fs::set_permissions(path, readonly_perms);
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(path, readonly_perms);
    //     // create_symlink_and_set_permissions

    //     // The `set_permissions*` should lint against the `create_dir_all`
    //     // below here and even during assignment to a variable.
    //     let path_two = Path::new("foo_two");
    //     let link_two = Path::new("baz_two");
    //     let _ = symlink_path(path_two, link_two);
    //     let _ = std::fs::set_permissions(path_two, readonly_perms);
    //     // create_symlink_and_set_permissions
    //     let _ = std::fs::set_permissions_nofollow(path_two, readonly_perms);
    //     // create_symlink_and_set_permissions

    //     symlink_path("bar", "symlink_foo");
    //     std::fs::set_permissions("bar", readonly_perms);
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow("bar", readonly_perms);
    //     // create_symlink_and_set_permissions

    //     // variable shadowing path_two that is not used by `create_dir_all`
    //     // should not trigger a lint
    //     let path_two = Path::new("foo_four");
    //     std::fs::set_permissions(path_two, readonly_perms);
    //     std::fs::set_permissions_nofollow(path_two, readonly_perms);
    // }

    // #[cfg(target_os = "windows")]
    // fn symlink_file_followed_by_set_perms() {
    //     #![feature(windows_permissions_ext)]
    //     use std::os::windows::fs::{PermissionsExt, symlink_file};

    //     let path = Path::new("foo");
    //     let link = Path::new("baz");
    //     symlink_file(path, link);
    //     std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     // The `set_permissions*` should lint against the `create_dir_all`
    //     // below here and even during assignment to a variable.
    //     let path_two = Path::new("foo_two");
    //     let link_two = Path::new("baz_two");
    //     let _ = symlink_file(path_two, link_two);
    //     let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     symlink_file("bar", "symlink_foo");
    //     std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions

    //     // variable shadowing path_two that is not used by `create_dir_all`
    //     // should not trigger a lint
    //     let path_two = Path::new("foo_four");
    //     std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
    //     std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    // }
    // ===============================================================================

    // ===============================================================================
    // The following tests linting `set_permissions*` against create_dir* and symlink*
    // with arguments passed by the parameters of the function it resides in
    fn create_dir_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir(&path);
        std::fs::set_permissions(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
    }

    fn create_dir_all_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(&path);
        std::fs::set_permissions(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
    }

    fn symlink_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
        use std::os::unix::fs::{PermissionsExt, symlink};

        symlink(&original, &link);
        std::fs::set_permissions(&original, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(&original, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
    }

    // #[cfg(target_os = "windows")]
    // fn symlink_dir_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
    //     #![feature(windows_permissions_ext)]
    //     use std::os::windows::fs::{PermissionsExt, symlink_dir};

    //     symlink_dir(&original, &link);
    //     std::fs::set_permissions(&original, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(&original, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    // }

    // #[cfg(target_os = "wasi")]
    // fn symlink_path_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
    //     use std::os::wasi::fs::symlink_path;

    //     let mut permissions = std::fs::metadata(&original).unwrap().permissions();
    //     permissions.set_readonly(true);

    //     symlink_path(&original, &link);
    //     std::fs::set_permissions(&original, permissions);
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(&original, permissions);
    //     // create_symlink_and_set_permissions
    // }

    // #[cfg(target_os = "windows")]
    // fn symlink_file_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
    //     #![feature(windows_permissions_ext)]
    //     use std::os::windows::fs::{PermissionsExt, symlink_file};

    //     symlink_file(&original, &link);
    //     std::fs::set_permissions(&original, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    //     std::fs::set_permissions_nofollow(&original, Permissions::from_file_attributes(0x2));
    //     // create_symlink_and_set_permissions
    // }

    // ===============================================================================

    // ===============================================================================
    // The following tests linting `set_permissions*` against `File::create`
    fn file_create_followed_by_set_perms() {
        use std::os::unix::fs::PermissionsExt;

        File::create("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }

    fn file_create_new_followed_by_set_perms() {
        use std::os::unix::fs::PermissionsExt;

        File::create_new("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }

    fn file_create_buffered_followed_by_set_perms() {
        use std::os::unix::fs::PermissionsExt;

        File::create_buffered("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }
    // ===============================================================================
}
