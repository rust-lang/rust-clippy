#[macro_export]
macro_rules! external_struct_rest_default {
    () => {
        #[derive(Default)]
        struct ExternalDefault {
            a: i32,
            b: i32,
        }

        let _ = ExternalDefault {
            a: 10,
            ..Default::default()
        };
    };
}
