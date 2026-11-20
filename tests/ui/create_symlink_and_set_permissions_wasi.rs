//@ only-target: wasi

// FIXME: Someone on wasi has to generate the .stderr files for these tests
#[cfg(target_os = "wasi")]
mod main_wasi {
    use std::os::wasi::fs::symlink_path;
    use std::path::Path;

    fn symlink_path_followed_by_set_perms() {
        let path = Path::new("foo");
        let link = Path::new("baz");
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);

        symlink_path(path, link);
        std::fs::set_permissions(path, readonly_perms);
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(path, readonly_perms);
        //~^ create_symlink_and_set_permissions

        // The `set_permissions*` should lint against the `create_dir_all`
        // below here and even during assignment to a variable.
        let path_two = Path::new("foo_two");
        let link_two = Path::new("baz_two");
        let _ = symlink_path(path_two, link_two);
        let _ = std::fs::set_permissions(path_two, readonly_perms);
        //~^ create_symlink_and_set_permissions
        let _ = std::fs::set_permissions_nofollow(path_two, readonly_perms);
        //~^ create_symlink_and_set_permissions

        symlink_path("bar", "symlink_foo");
        std::fs::set_permissions("bar", readonly_perms);
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow("bar", readonly_perms);
        //~^ create_symlink_and_set_permissions

        // variable shadowing path_two that is not used by `create_dir_all`
        // should not trigger a lint
        let path_two = Path::new("foo_four");
        std::fs::set_permissions(path_two, readonly_perms);
        std::fs::set_permissions_nofollow(path_two, readonly_perms);
    }

    fn symlink_path_from_arg_followed_by_set_perms<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
        let mut permissions = std::fs::metadata(&original).unwrap().permissions();
        permissions.set_readonly(true);

        symlink_path(&original, &link);
        std::fs::set_permissions(&link, permissions);
        //~^ create_symlink_and_set_permissions
        std::fs::set_permissions_nofollow(&link, permissions);
        //~^ create_symlink_and_set_permissions
    }
}
