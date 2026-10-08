#![warn(clippy::create_dir_and_set_permissions)]
#![feature(set_permissions_nofollow)]

#[cfg(target_family = "unix")]
mod main_unix {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn create_dir_followed_by_set_perms() {
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

    fn create_dir_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
        std::fs::create_dir(&path);
        std::fs::set_permissions(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
    }

    fn create_dir_all_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
        std::fs::create_dir_all(&path);
        std::fs::set_permissions(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
        std::fs::set_permissions_nofollow(&path, Permissions::from_mode(0o700));
        //~^ create_dir_and_set_permissions
    }
}

// FIXME: Figure out how to get this test to work on Windows
// #[cfg(target_os = "windows")]
// mod main_windows {
//     #![feature(windows_permissions_ext)]
//     use std::fs::Permissions;
//     use std::os::windows::fs::PermissionsExt;
//     use std::path::Path;

//     fn create_dir_followed_by_set_perms() {
//         let path = Path::new("foo");
//         std::fs::create_dir(path);

//         std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         // The `set_permissions*` should lint against the `create_dir`
//         // below here and even during assignment to a variable.
//         let path_two = Path::new("foo_two");
//         let _ = std::fs::create_dir(path_two);
//         let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         std::fs::create_dir("bar");
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         // variable shadowing path_two that is not used by `create_dir`
//         // should not trigger a lint
//         let path_two = Path::new("foo_four");
//         std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//     }

//     fn create_dir_all_followed_by_set_perms() {
//         let path = Path::new("foo");
//         std::fs::create_dir_all(path);
//         std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         // The `set_permissions*` should lint against the `create_dir_all`
//         // below here and even during assignment to a variable.
//         let path_two = Path::new("foo_two");
//         let _ = std::fs::create_dir_all(path_two);
//         let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         std::fs::create_dir_all("bar");
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions

//         // variable shadowing path_two that is not used by `create_dir_all`
//         // should not trigger a lint
//         let path_two = Path::new("foo_four");
//         std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//     }

//     fn create_dir_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
//         std::fs::create_dir(&path);
//         std::fs::set_permissions(&path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow(&path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//     }

//     fn create_dir_all_from_arg_followed_by_set_perms<P: AsRef<Path>>(path: P) {
//         std::fs::create_dir_all(&path);
//         std::fs::set_permissions(&path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//         std::fs::set_permissions_nofollow(&path, Permissions::from_file_attributes(0x2));
//         // create_dir_and_set_permissions
//     }
// }
