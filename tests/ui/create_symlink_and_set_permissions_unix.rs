//@ only-target: linux

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
