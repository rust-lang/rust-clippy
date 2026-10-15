#![warn(clippy::create_regular_file_and_set_permissions)]
#![feature(file_buffered)]
#![feature(set_permissions_nofollow)]

#[cfg(target_family = "unix")]
mod main_unix {
    use std::fs::{File, Permissions};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn no_prior_file_creation_calls() {
        std::fs::set_permissions("bar_two", Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow("bar_two", Permissions::from_mode(0o700));

        let path_three = Path::new("foo_three");
        std::fs::set_permissions(path_three, Permissions::from_mode(0o700));
        std::fs::set_permissions_nofollow(path_three, Permissions::from_mode(0o700));
    }

    fn file_create_followed_by_set_perms() {
        File::create("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }

    fn file_create_new_followed_by_set_perms() {
        File::create_new("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }

    fn file_create_buffered_followed_by_set_perms() {
        File::create_buffered("bar").unwrap();
        std::fs::set_permissions("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_mode(0o700));
        //~^ create_regular_file_and_set_permissions
    }
}

// FIXME: Figure out how to get this test to work on Windows
// #[cfg(target_os = "windows")]
// mod main_windows {
//     #![feature(windows_permissions_ext)]
//     use std::fs::{File, Permissions};
//     use std::os::windows::fs::PermissionsExt;
//     use std::path::Path;

//     fn no_prior_file_creation_calls() {
//         const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
//         std::fs::set_permissions("bar_two", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         std::fs::set_permissions_nofollow("bar_two", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));

//         let path_three = Path::new("foo_three");
//         std::fs::set_permissions(path_three, Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         std::fs::set_permissions_nofollow(path_three, Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//     }

//     fn file_create_followed_by_set_perms() {
//         const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
//         File::create("bar").unwrap();
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//     }

//     fn file_create_new_followed_by_set_perms() {
//         const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
//         File::create_new("bar").unwrap();
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//     }

//     fn file_create_buffered_followed_by_set_perms() {
//         const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
//         File::create_buffered("bar").unwrap();
//         std::fs::set_permissions("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//         std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(FILE_ATTRIBUTE_HIDDEN));
//         // create_regular_file_and_set_permissions
//     }
// }
