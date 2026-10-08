#![warn(clippy::create_symlink_and_set_permissions)]
#![feature(set_permissions_nofollow)]

#[cfg(target_family = "unix")]
mod main_unix {
    use std::fs::Permissions;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::Path;

    fn symlink_followed_by_set_perms() {
        let path = Path::new("foo");
        let link = Path::new("baz");
        symlink(path, link);
        std::fs::set_permissions(link, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(link, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions

        // The `set_permissions*` should lint against the `symlink`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let link_two = Path::new("baz_two");
        let _ = symlink(path_two, link_two);
        let _ = std::fs::set_permissions(link_two, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(link_two, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions

        symlink("bar", "symlink_foo");
        std::fs::set_permissions("symlink_foo", Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow("symlink_foo", Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
    }

    fn symlink_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
        symlink(&original, &link);
        std::fs::set_permissions(&link, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(&link, Permissions::from_mode(0o700));
        //~^ create_symlink_and_set_permissions
    }
}

// FIXME: Figure out how to get this test to work on Windows and wasi
// #[cfg(target_os = "windows")]
// mod main_windows {
//     #![feature(windows_permissions_ext)]
//     use std::fs::Permissions;
//     use std::os::windows::fs::{PermissionsExt, symlink_dir, symlink_file};
//     use std::path::Path;

//     fn symlink_file_followed_by_set_perms() {
//         let path = Path::new("foo");
//         let link = Path::new("baz");
//         symlink_file(path, link);
//         std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         // The `set_permissions*` should lint against the `create_dir_all`
//         // below here and even during assignment to a variable.
//         let path_two = Path::new("foo_two");
//         let link_two = Path::new("baz_two");
//         let _ = symlink_file(path_two, link_two);
//         let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         symlink_file("bar", "symlink_foo");
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         // variable shadowing path_two that is not used by `create_dir_all`
//         // should not trigger a lint
//         let path_two = Path::new("foo_four");
//         std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//     }

//     fn symlink_dir_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
//         symlink_dir(&original, &link);
//         std::fs::set_permissions(&link, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(&link, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//     }

//     fn symlink_dir_followed_by_set_perms() {
//         let path = Path::new("foo");
//         let link = Path::new("baz");
//         symlink_dir(path, link);

//         std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         // The `set_permissions*` should lint against the `create_dir_all`
//         // below here and even during assignment to a variable.
//         let path_two = Path::new("foo_two");
//         let link_two = Path::new("baz_two");
//         let _ = symlink_dir(path_two, link_two);
//         let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         symlink_dir("bar", "symlink_foo");
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions

//         // variable shadowing path_two that is not used by `create_dir_all`
//         // should not trigger a lint
//         let path_two = Path::new("foo_four");
//         std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
//         std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
//     }

//     fn symlink_file_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
//         symlink_file(&original, &link);
//         std::fs::set_permissions(&link, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(&link, Permissions::from_file_attributes(0x2));
//         // create_symlink_and_set_permissions
//     }
// }

// #[cfg(target_os = "wasi")]
// mod main_wasi {
//     use std::os::wasi::fs::symlink_path;
//     use std::path::Path;

//     fn symlink_path_followed_by_set_perms() {
//         let path = Path::new("foo");
//         let link = Path::new("baz");
//         let mut permissions = std::fs::metadata(&path).unwrap().permissions();
//         permissions.set_readonly(true);

//         symlink_path(path, link);
//         std::fs::set_permissions(path, readonly_perms);
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(path, readonly_perms);
//         // create_symlink_and_set_permissions

//         // The `set_permissions*` should lint against the `create_dir_all`
//         // below here and even during assignment to a variable.
//         let path_two = Path::new("foo_two");
//         let link_two = Path::new("baz_two");
//         let _ = symlink_path(path_two, link_two);
//         let _ = std::fs::set_permissions(path_two, readonly_perms);
//         // create_symlink_and_set_permissions
//         let _ = std::fs::set_permissions_nofollow(path_two, readonly_perms);
//         // create_symlink_and_set_permissions

//         symlink_path("bar", "symlink_foo");
//         std::fs::set_permissions("bar", readonly_perms);
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", readonly_perms);
//         // create_symlink_and_set_permissions

//         // variable shadowing path_two that is not used by `create_dir_all`
//         // should not trigger a lint
//         let path_two = Path::new("foo_four");
//         std::fs::set_permissions(path_two, readonly_perms);
//         std::fs::set_permissions_nofollow(path_two, readonly_perms);
//     }

//     fn symlink_path_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
//         let mut permissions = std::fs::metadata(&original).unwrap().permissions();
//         permissions.set_readonly(true);

//         symlink_path(&original, &link);
//         std::fs::set_permissions(&link, permissions);
//         // create_symlink_and_set_permissions
//         std::fs::set_permissions_nofollow(&link, permissions);
//         // create_symlink_and_set_permissions
//     }
// }
