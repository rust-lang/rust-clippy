//@ only-target: windows

// FIXME: Someone on Windows has to generate the .stderr files for these tests
#[cfg(target_os = "windows")]
mod main_windows {
    #![feature(windows_permissions_ext)]
    use std::fs::Permissions;
    use std::os::windows::fs::{PermissionsExt, symlink_dir, symlink_file};
    use std::path::Path;

    fn symlink_file_followed_by_set_perms() {
        let path = Path::new("foo");
        let link = Path::new("baz");
        symlink_file(path, link);
        std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir_all`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let link_two = Path::new("baz_two");
        let _ = symlink_file(path_two, link_two);
        let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        symlink_file("bar", "symlink_foo");
        std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir_all`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
        std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    }

    fn symlink_dir_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
        symlink_dir(&original, &link);
        std::fs::set_permissions(&link, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(&link, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
    }

    fn symlink_dir_followed_by_set_perms() {
        let path = Path::new("foo");
        let link = Path::new("baz");
        symlink_dir(path, link);

        std::fs::set_permissions(path, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(path, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir_all`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let link_two = Path::new("baz_two");
        let _ = symlink_dir(path_two, link_two);
        let _ = std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        symlink_dir("bar", "symlink_foo");
        std::fs::set_permissions("bar", Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow("bar", Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir_all`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, Permissions::from_file_attributes(0x2));
        std::fs::set_permissions_nofollow(path_two, Permissions::from_file_attributes(0x2));
    }

    fn symlink_file_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
        symlink_file(&original, &link);
        std::fs::set_permissions(&link, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(&link, Permissions::from_file_attributes(0x2));
        //~^ create_symlink_and_set_permissions
    }
}
