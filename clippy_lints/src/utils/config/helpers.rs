use serde::Deserialize;

macro_rules! define_Conf {
    ($(#[$doc:meta] ($config:ident, $config_str:literal: $Ty:ty, $default:expr),)+) => {
        /// Type used to store lint configuration.
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case", deny_unknown_fields)]
        pub struct Conf {
            $(
                #[$doc]
                #[serde(default = $config_str)]
                #[serde(with = $config_str)]
                pub $config: $Ty,
            )+
            #[allow(dead_code)]
            #[serde(default)]
            third_party: Option<::toml::Value>,
        }

        $(
            mod $config {
                use serde::Deserialize;
                crate fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<$Ty, D::Error> {
                    use super::super::{ERRORS, Error};
                    Ok(
                        <$Ty>::deserialize(deserializer).unwrap_or_else(|e| {
                            ERRORS
                                .lock()
                                .expect("no threading here")
                                .push(Error::Toml(e.to_string()));
                            super::$config()
                        })
                    )
                }
            }

            #[must_use]
            fn $config() -> $Ty {
                let x = $default;
                x
            }
        )+
    };
}

define_Conf! {
    /// Lint: BLACKLISTED_NAME. The list of blacklisted names to lint about
    (blacklisted_names, "blacklisted_names": Vec<String>, ["foo", "bar", "baz", "quux"].iter().map(ToString::to_string).collect()),
    /// Lint: COGNITIVE_COMPLEXITY. The maximum cognitive complexity a function can have
    (cognitive_complexity_threshold, "cognitive_complexity_threshold": u64, 25),
    /// DEPRECATED LINT: CYCLOMATIC_COMPLEXITY. Use the Cognitive Complexity lint instead.
    (cyclomatic_complexity_threshold, "cyclomatic_complexity_threshold": Option<u64>, None),
    /// Lint: DOC_MARKDOWN. The list of words this lint should not consider as identifiers needing ticks
    (doc_valid_idents, "doc_valid_idents": Vec<String>, [
        "KiB", "MiB", "GiB", "TiB", "PiB", "EiB",
        "DirectX",
        "ECMAScript",
        "GPLv2", "GPLv3",
        "GitHub", "GitLab",
        "IPv4", "IPv6",
        "JavaScript",
        "NaN", "NaNs",
        "OAuth",
        "OpenGL", "OpenSSH", "OpenSSL", "OpenStreetMap",
        "TrueType",
        "iOS", "macOS",
        "TeX", "LaTeX", "BibTeX", "BibLaTeX",
        "MinGW",
        "CamelCase",
    ].iter().map(ToString::to_string).collect()),
    /// Lint: TOO_MANY_ARGUMENTS. The maximum number of argument a function or method can have
    (too_many_arguments_threshold, "too_many_arguments_threshold": u64, 7),
    /// Lint: TYPE_COMPLEXITY. The maximum complexity a type can have
    (type_complexity_threshold, "type_complexity_threshold": u64, 250),
    /// Lint: MANY_SINGLE_CHAR_NAMES. The maximum number of single char bindings a scope may have
    (single_char_binding_names_threshold, "single_char_binding_names_threshold": u64, 5),
    /// Lint: BOXED_LOCAL. The maximum size of objects (in bytes) that will be linted. Larger objects are ok on the heap
    (too_large_for_stack, "too_large_for_stack": u64, 200),
    /// Lint: ENUM_VARIANT_NAMES. The minimum number of enum variants for the lints about variant names to trigger
    (enum_variant_name_threshold, "enum_variant_name_threshold": u64, 3),
    /// Lint: LARGE_ENUM_VARIANT. The maximum size of a enum's variant to avoid box suggestion
    (enum_variant_size_threshold, "enum_variant_size_threshold": u64, 200),
    /// Lint: VERBOSE_BIT_MASK. The maximum allowed size of a bit mask before suggesting to use 'trailing_zeros'
    (verbose_bit_mask_threshold, "verbose_bit_mask_threshold": u64, 1),
    /// Lint: DECIMAL_LITERAL_REPRESENTATION. The lower bound for linting decimal literals
    (literal_representation_threshold, "literal_representation_threshold": u64, 16384),
    /// Lint: TRIVIALLY_COPY_PASS_BY_REF. The maximum size (in bytes) to consider a `Copy` type for passing by value instead of by reference.
    (trivial_copy_size_limit, "trivial_copy_size_limit": Option<u64>, None),
    /// Lint: TOO_MANY_LINES. The maximum number of lines a function or method can have
    (too_many_lines_threshold, "too_many_lines_threshold": u64, 100),
    /// Lint: LARGE_STACK_ARRAYS. The maximum allowed size for arrays on the stack
    (array_size_threshold, "array_size_threshold": u64, 512_000),
}

impl Default for Conf {
    #[must_use]
    fn default() -> Self {
        toml::from_str("").expect("we never error on empty config files")
    }
}
