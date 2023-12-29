use crate::msrvs::Msrv;
use crate::types::{DisallowedPath, MacroMatcher, MatchLintBehaviour, Rename};
use crate::ClippyConfiguration;
use rustc_data_structures::fx::FxHashSet;
use rustc_session::Session;
use rustc_span::{BytePos, Pos, SourceFile, Span, SyntaxContext};
use serde::de::{IgnoredAny, IntoDeserializer, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;
use std::{cmp, env, fmt, fs, io};

#[rustfmt::skip]
const DEFAULT_DOC_VALID_IDENTS: &[&str] = &[
    "KiB", "MiB", "GiB", "TiB", "PiB", "EiB",
    "DirectX",
    "ECMAScript",
    "GPLv2", "GPLv3",
    "GitHub", "GitLab",
    "IPv4", "IPv6",
    "ClojureScript", "CoffeeScript", "JavaScript", "PureScript", "TypeScript",
    "WebAssembly",
    "NaN", "NaNs",
    "OAuth", "GraphQL",
    "OCaml",
    "OpenGL", "OpenMP", "OpenSSH", "OpenSSL", "OpenStreetMap", "OpenDNS",
    "WebGL", "WebGL2", "WebGPU",
    "TensorFlow",
    "TrueType",
    "iOS", "macOS", "FreeBSD",
    "TeX", "LaTeX", "BibTeX", "BibLaTeX",
    "MinGW",
    "CamelCase",
];
const DEFAULT_DISALLOWED_NAMES: &[&str] = &["foo", "baz", "quux"];
const DEFAULT_ALLOWED_IDENTS_BELOW_MIN_CHARS: &[&str] = &["i", "j", "x", "y", "z", "w", "n"];

/// Conf with parse errors
#[derive(Default)]
struct TryConf {
    conf: Conf,
    errors: Vec<ConfError>,
    warnings: Vec<ConfError>,
}

impl TryConf {
    fn from_toml_error(file: &SourceFile, error: &toml::de::Error) -> Self {
        Self {
            conf: Conf::default(),
            errors: vec![ConfError::from_toml(file, error)],
            warnings: vec![],
        }
    }
}

#[derive(Debug)]
struct ConfError {
    message: String,
    span: Span,
}

impl ConfError {
    fn from_toml(file: &SourceFile, error: &toml::de::Error) -> Self {
        let span = error.span().unwrap_or(0..file.source_len.0 as usize);
        Self::spanned(file, error.message(), span)
    }

    fn spanned(file: &SourceFile, message: impl Into<String>, span: Range<usize>) -> Self {
        Self {
            message: message.into(),
            span: Span::new(
                file.start_pos + BytePos::from_usize(span.start),
                file.start_pos + BytePos::from_usize(span.end),
                SyntaxContext::root(),
                None,
            ),
        }
    }
}

macro_rules! wrap_option {
    () => {
        None
    };
    ($x:literal) => {
        Some($x)
    };
}

macro_rules! default_text {
    ($value:expr) => {{
        let mut text = String::new();
        $value.serialize(toml::ser::ValueSerializer::new(&mut text)).unwrap();
        text
    }};
    ($value:expr, $override:expr) => {
        $override.to_string()
    };
}

macro_rules! define_Conf {
    ($(
        $(#[doc = $doc:literal])+
        $(#[conf_deprecated($dep:literal, $new_conf:ident)])?
        $(#[default_text = $default_text:expr])?
        ($name:ident: $ty:ty = $default:expr),
    )*) => {
        /// Clippy lint configuration
        pub struct Conf {
            $($(#[doc = $doc])+ pub $name: $ty,)*
        }

        mod defaults {
            use super::*;
            $(pub fn $name() -> $ty { $default })*
        }

        impl Default for Conf {
            fn default() -> Self {
                Self { $($name: defaults::$name(),)* }
            }
        }

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[allow(non_camel_case_types)]
        enum Field { $($name,)* third_party, }

        struct ConfVisitor<'a>(&'a SourceFile);

        impl<'de> Visitor<'de> for ConfVisitor<'_> {
            type Value = TryConf;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("Conf")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error> where V: MapAccess<'de> {
                let mut errors = Vec::new();
                let mut warnings = Vec::new();
                $(let mut $name = None;)*
                // could get `Field` here directly, but get `String` first for diagnostics
                while let Some(name) = map.next_key::<toml::Spanned<String>>()? {
                    match Field::deserialize(name.get_ref().as_str().into_deserializer()) {
                        Err(e) => {
                            let e: FieldError = e;
                            errors.push(ConfError::spanned(self.0, e.0, name.span()));
                        }
                        $(Ok(Field::$name) => {
                            $(warnings.push(ConfError::spanned(self.0, format!("deprecated field `{}`. {}", name.get_ref(), $dep), name.span()));)?
                            let raw_value = map.next_value::<toml::Spanned<toml::Value>>()?;
                            let value_span = raw_value.span();
                            match <$ty>::deserialize(raw_value.into_inner()) {
                                Err(e) => errors.push(ConfError::spanned(self.0, e.to_string().replace('\n', " ").trim(), value_span)),
                                Ok(value) => match $name {
                                    Some(_) => errors.push(ConfError::spanned(self.0, format!("duplicate field `{}`", name.get_ref()), name.span())),
                                    None => {
                                        $name = Some(value);
                                        // $new_conf is the same as one of the defined `$name`s, so
                                        // this variable is defined in line 2 of this function.
                                        $(match $new_conf {
                                            Some(_) => errors.push(ConfError::spanned(self.0, concat!(
                                                "duplicate field `", stringify!($new_conf),
                                                "` (provided as `", stringify!($name), "`)"
                                            ), name.span())),
                                            None => $new_conf = $name.clone(),
                                        })?
                                    },
                                }
                            }
                        })*
                        // ignore contents of the third_party key
                        Ok(Field::third_party) => drop(map.next_value::<IgnoredAny>())
                    }
                }
                let conf = Conf { $($name: $name.unwrap_or_else(defaults::$name),)* };
                Ok(TryConf { conf, errors, warnings })
            }
        }

        pub fn get_configuration_metadata() -> Vec<ClippyConfiguration> {
            vec![
                $(
                    {
                        let deprecation_reason = wrap_option!($($dep)?);

                        ClippyConfiguration::new(
                            stringify!($name),
                            default_text!(defaults::$name() $(, $default_text)?),
                            concat!($($doc, '\n',)*),
                            deprecation_reason,
                        )
                    },
                )+
            ]
        }
    };
}

define_Conf! {
    /// Lint: ARITHMETIC_SIDE_EFFECTS.
    ///
    /// Suppress checking of the passed type names in all types of operations.
    ///
    /// If a specific operation is desired, consider using `arithmetic_side_effects_allowed_binary` or `arithmetic_side_effects_allowed_unary` instead.
    ///
    /// #### Example
    ///
    /// ```toml
    /// arithmetic-side-effects-allowed = ["SomeType", "AnotherType"]
    /// ```
    ///
    /// #### Noteworthy
    ///
    /// A type, say `SomeType`, listed in this configuration has the same behavior of
    /// `["SomeType" , "*"], ["*", "SomeType"]` in `arithmetic_side_effects_allowed_binary`.
    (arithmetic_side_effects_allowed: FxHashSet<String> = <_>::default()),
    /// Lint: ARITHMETIC_SIDE_EFFECTS.
    ///
    /// Suppress checking of the passed type pair names in binary operations like addition or
    /// multiplication.
    ///
    /// Supports the "*" wildcard to indicate that a certain type won't trigger the lint regardless
    /// of the involved counterpart. For example, `["SomeType", "*"]` or `["*", "AnotherType"]`.
    ///
    /// Pairs are asymmetric, which means that `["SomeType", "AnotherType"]` is not the same as
    /// `["AnotherType", "SomeType"]`.
    ///
    /// #### Example
    ///
    /// ```toml
    /// arithmetic-side-effects-allowed-binary = [["SomeType" , "f32"], ["AnotherType", "*"]]
    /// ```
    (arithmetic_side_effects_allowed_binary: Vec<[String; 2]> = <_>::default()),
    /// Lint: ARITHMETIC_SIDE_EFFECTS.
    ///
    /// Suppress checking of the passed type names in unary operations like "negation" (`-`).
    ///
    /// #### Example
    ///
    /// ```toml
    /// arithmetic-side-effects-allowed-unary = ["SomeType", "AnotherType"]
    /// ```
    (arithmetic_side_effects_allowed_unary: FxHashSet<String> = <_>::default()),
    /// Lint: ENUM_VARIANT_NAMES, LARGE_TYPES_PASSED_BY_VALUE, TRIVIALLY_COPY_PASS_BY_REF, UNNECESSARY_WRAPS, UNUSED_SELF, UPPER_CASE_ACRONYMS, WRONG_SELF_CONVENTION, BOX_COLLECTION, REDUNDANT_ALLOCATION, RC_BUFFER, VEC_BOX, OPTION_OPTION, LINKEDLIST, RC_MUTEX, UNNECESSARY_BOX_RETURNS, SINGLE_CALL_FN.
    ///
    /// Suppress lints whenever the suggested change would cause breakage for other crates.
    (avoid_breaking_exported_api: bool = true),
    /// Lint: MANUAL_SPLIT_ONCE, MANUAL_STR_REPEAT, CLONED_INSTEAD_OF_COPIED, REDUNDANT_FIELD_NAMES, OPTION_MAP_UNWRAP_OR, REDUNDANT_STATIC_LIFETIMES, FILTER_MAP_NEXT, CHECKED_CONVERSIONS, MANUAL_RANGE_CONTAINS, USE_SELF, MEM_REPLACE_WITH_DEFAULT, MANUAL_NON_EXHAUSTIVE, OPTION_AS_REF_DEREF, MAP_UNWRAP_OR, MATCH_LIKE_MATCHES_MACRO, MANUAL_STRIP, MISSING_CONST_FOR_FN, UNNESTED_OR_PATTERNS, FROM_OVER_INTO, PTR_AS_PTR, IF_THEN_SOME_ELSE_NONE, APPROX_CONSTANT, DEPRECATED_CFG_ATTR, INDEX_REFUTABLE_SLICE, MAP_CLONE, BORROW_AS_PTR, MANUAL_BITS, ERR_EXPECT, CAST_ABS_TO_UNSIGNED, UNINLINED_FORMAT_ARGS, MANUAL_CLAMP, MANUAL_LET_ELSE, UNCHECKED_DURATION_SUBTRACTION, COLLAPSIBLE_STR_REPLACE, SEEK_FROM_CURRENT, SEEK_REWIND, UNNECESSARY_LAZY_EVALUATIONS, TRANSMUTE_PTR_TO_REF, ALMOST_COMPLETE_RANGE, NEEDLESS_BORROW, DERIVABLE_IMPLS, MANUAL_IS_ASCII_CHECK, MANUAL_REM_EUCLID, MANUAL_RETAIN, TYPE_REPETITION_IN_BOUNDS, TUPLE_ARRAY_CONVERSIONS, MANUAL_TRY_FOLD, MANUAL_HASH_ONE, ITER_KV_MAP.
    ///
    /// The minimum rust version that the project supports. Defaults to the `rust-version` field in `Cargo.toml`
    #[default_text = ""]
    (msrv: Msrv = Msrv::empty()),
    /// DEPRECATED LINT: BLACKLISTED_NAME.
    ///
    /// Use the Disallowed Names lint instead
    #[conf_deprecated("Please use `disallowed-names` instead", disallowed_names)]
    (blacklisted_names: Vec<String> = Vec::new()),
    /// Lint: COGNITIVE_COMPLEXITY.
    ///
    /// The maximum cognitive complexity a function can have
    (cognitive_complexity_threshold: u64 = 25),
    /// Lint: EXCESSIVE_NESTING.
    ///
    /// The maximum amount of nesting a block can reside in
    (excessive_nesting_threshold: u64 = 0),
    /// DEPRECATED LINT: CYCLOMATIC_COMPLEXITY.
    ///
    /// Use the Cognitive Complexity lint instead.
    #[conf_deprecated("Please use `cognitive-complexity-threshold` instead", cognitive_complexity_threshold)]
    (cyclomatic_complexity_threshold: u64 = 25),
    /// Lint: DISALLOWED_NAMES.
    ///
    /// The list of disallowed names to lint about. NB: `bar` is not here since it has legitimate uses. The value
    /// `".."` can be used as part of the list to indicate that the configured values should be appended to the
    /// default configuration of Clippy. By default, any configuration will replace the default value.
    (disallowed_names: Vec<String> = DEFAULT_DISALLOWED_NAMES.iter().map(ToString::to_string).collect()),
    /// Lint: SEMICOLON_INSIDE_BLOCK.
    ///
    /// Whether to lint only if it's multiline.
    (semicolon_inside_block_ignore_singleline: bool = false),
    /// Lint: SEMICOLON_OUTSIDE_BLOCK.
    ///
    /// Whether to lint only if it's singleline.
    (semicolon_outside_block_ignore_multiline: bool = false),
    /// Lint: DOC_MARKDOWN.
    ///
    /// The list of words this lint should not consider as identifiers needing ticks. The value
    /// `".."` can be used as part of the list to indicate, that the configured values should be appended to the
    /// default configuration of Clippy. By default, any configuration will replace the default value. For example:
    /// * `doc-valid-idents = ["ClipPy"]` would replace the default list with `["ClipPy"]`.
    /// * `doc-valid-idents = ["ClipPy", ".."]` would append `ClipPy` to the default list.
    (doc_valid_idents: Vec<String> = DEFAULT_DOC_VALID_IDENTS.iter().map(ToString::to_string).collect()),
    /// Lint: TOO_MANY_ARGUMENTS.
    ///
    /// The maximum number of argument a function or method can have
    (too_many_arguments_threshold: u64 = 7),
    /// Lint: TYPE_COMPLEXITY.
    ///
    /// The maximum complexity a type can have
    (type_complexity_threshold: u64 = 250),
    /// Lint: MANY_SINGLE_CHAR_NAMES.
    ///
    /// The maximum number of single char bindings a scope may have
    (single_char_binding_names_threshold: u64 = 4),
    /// Lint: BOXED_LOCAL, USELESS_VEC.
    ///
    /// The maximum size of objects (in bytes) that will be linted. Larger objects are ok on the heap
    (too_large_for_stack: u64 = 200),
    /// Lint: ENUM_VARIANT_NAMES.
    ///
    /// The minimum number of enum variants for the lints about variant names to trigger
    (enum_variant_name_threshold: u64 = 3),
    /// Lint: STRUCT_VARIANT_NAMES.
    ///
    /// The minimum number of struct fields for the lints about field names to trigger
    (struct_field_name_threshold: u64 = 3),
    /// Lint: LARGE_ENUM_VARIANT.
    ///
    /// The maximum size of an enum's variant to avoid box suggestion
    (enum_variant_size_threshold: u64 = 200),
    /// Lint: VERBOSE_BIT_MASK.
    ///
    /// The maximum allowed size of a bit mask before suggesting to use 'trailing_zeros'
    (verbose_bit_mask_threshold: u64 = 1),
    /// Lint: DECIMAL_LITERAL_REPRESENTATION.
    ///
    /// The lower bound for linting decimal literals
    (literal_representation_threshold: u64 = 16384),
    /// Lint: TRIVIALLY_COPY_PASS_BY_REF.
    ///
    /// The maximum size (in bytes) to consider a `Copy` type for passing by value instead of by
    /// reference. By default there is no limit
    #[default_text = ""]
    (trivial_copy_size_limit: Option<u64> = None),
    /// Lint: LARGE_TYPES_PASSED_BY_VALUE.
    ///
    /// The minimum size (in bytes) to consider a type for passing by reference instead of by value.
    (pass_by_value_size_limit: u64 = 256),
    /// Lint: TOO_MANY_LINES.
    ///
    /// The maximum number of lines a function or method can have
    (too_many_lines_threshold: u64 = 100),
    /// Lint: LARGE_STACK_ARRAYS, LARGE_CONST_ARRAYS.
    ///
    /// The maximum allowed size for arrays on the stack
    (array_size_threshold: u64 = 512_000),
    /// Lint: LARGE_STACK_FRAMES.
    ///
    /// The maximum allowed stack size for functions in bytes
    (stack_size_threshold: u64 = 512_000),
    /// Lint: VEC_BOX.
    ///
    /// The size of the boxed type in bytes, where boxing in a `Vec` is allowed
    (vec_box_size_threshold: u64 = 4096),
    /// Lint: TYPE_REPETITION_IN_BOUNDS.
    ///
    /// The maximum number of bounds a trait can have to be linted
    (max_trait_bounds: u64 = 3),
    /// Lint: STRUCT_EXCESSIVE_BOOLS.
    ///
    /// The maximum number of bool fields a struct can have
    (max_struct_bools: u64 = 3),
    /// Lint: FN_PARAMS_EXCESSIVE_BOOLS.
    ///
    /// The maximum number of bool parameters a function can have
    (max_fn_params_bools: u64 = 3),
    /// Lint: WILDCARD_IMPORTS.
    ///
    /// Whether to allow certain wildcard imports (prelude, super in tests).
    (warn_on_all_wildcard_imports: bool = false),
    /// Lint: DISALLOWED_MACROS.
    ///
    /// The list of disallowed macros, written as fully qualified paths.
    (disallowed_macros: Vec<DisallowedPath> = Vec::new()),
    /// Lint: DISALLOWED_METHODS.
    ///
    /// The list of disallowed methods, written as fully qualified paths.
    (disallowed_methods: Vec<DisallowedPath> = Vec::new()),
    /// Lint: DISALLOWED_TYPES.
    ///
    /// The list of disallowed types, written as fully qualified paths.
    (disallowed_types: Vec<DisallowedPath> = Vec::new()),
    /// Lint: UNREADABLE_LITERAL.
    ///
    /// Should the fraction of a decimal be linted to include separators.
    (unreadable_literal_lint_fractions: bool = true),
    /// Lint: UPPER_CASE_ACRONYMS.
    ///
    /// Enables verbose mode. Triggers if there is more than one uppercase char next to each other
    (upper_case_acronyms_aggressive: bool = false),
    /// Lint: MANUAL_LET_ELSE.
    ///
    /// Whether the matches should be considered by the lint, and whether there should
    /// be filtering for common types.
    (matches_for_let_else: MatchLintBehaviour = MatchLintBehaviour::WellKnownTypes),
    /// Lint: CARGO_COMMON_METADATA.
    ///
    /// For internal testing only, ignores the current `publish` settings in the Cargo manifest.
    (cargo_ignore_publish: bool = false),
    /// Lint: NONSTANDARD_MACRO_BRACES.
    ///
    /// Enforce the named macros always use the braces specified.
    ///
    /// A `MacroMatcher` can be added like so `{ name = "macro_name", brace = "(" }`. If the macro
    /// could be used with a full path two `MacroMatcher`s have to be added one with the full path
    /// `crate_name::macro_name` and one with just the macro name.
    (standard_macro_braces: Vec<MacroMatcher> = Vec::new()),
    /// Lint: MISSING_ENFORCED_IMPORT_RENAMES.
    ///
    /// The list of imports to always rename, a fully qualified path followed by the rename.
    (enforced_import_renames: Vec<Rename> = Vec::new()),
    /// Lint: DISALLOWED_SCRIPT_IDENTS.
    ///
    /// The list of unicode scripts allowed to be used in the scope.
    (allowed_scripts: Vec<String> = vec!["Latin".to_string()]),
    /// Lint: NON_SEND_FIELDS_IN_SEND_TY.
    ///
    /// Whether to apply the raw pointer heuristic to determine if a type is `Send`.
    (enable_raw_pointer_heuristic_for_send: bool = true),
    /// Lint: INDEX_REFUTABLE_SLICE.
    ///
    /// When Clippy suggests using a slice pattern, this is the maximum number of elements allowed in
    /// the slice pattern that is suggested. If more elements are necessary, the lint is suppressed.
    /// For example, `[_, _, _, e, ..]` is a slice pattern with 4 elements.
    (max_suggested_slice_pattern_length: u64 = 3),
    /// Lint: AWAIT_HOLDING_INVALID_TYPE.
    (await_holding_invalid_types: Vec<DisallowedPath> = Vec::new()),
    /// Lint: LARGE_INCLUDE_FILE.
    ///
    /// The maximum size of a file included via `include_bytes!()` or `include_str!()`, in bytes
    (max_include_file_size: u64 = 1_000_000),
    /// Lint: EXPECT_USED.
    ///
    /// Whether `expect` should be allowed in test functions or `#[cfg(test)]`
    (allow_expect_in_tests: bool = false),
    /// Lint: UNWRAP_USED.
    ///
    /// Whether `unwrap` should be allowed in test functions or `#[cfg(test)]`
    (allow_unwrap_in_tests: bool = false),
    /// Lint: DBG_MACRO.
    ///
    /// Whether `dbg!` should be allowed in test functions or `#[cfg(test)]`
    (allow_dbg_in_tests: bool = false),
    /// Lint: PRINT_STDOUT, PRINT_STDERR.
    ///
    /// Whether print macros (ex. `println!`) should be allowed in test functions or `#[cfg(test)]`
    (allow_print_in_tests: bool = false),
    /// Lint: RESULT_LARGE_ERR.
    ///
    /// The maximum size of the `Err`-variant in a `Result` returned from a function
    (large_error_threshold: u64 = 128),
    /// Lint: MUTABLE_KEY_TYPE, IFS_SAME_COND.
    ///
    /// A list of paths to types that should be treated like `Arc`, i.e. ignored but
    /// for the generic parameters for determining interior mutability
    (ignore_interior_mutability: Vec<String> = Vec::from(["bytes::Bytes".into()])),
    /// Lint: UNINLINED_FORMAT_ARGS.
    ///
    /// Whether to allow mixed uninlined format args, e.g. `format!("{} {}", a, foo.bar)`
    (allow_mixed_uninlined_format_args: bool = true),
    /// Lint: INDEXING_SLICING.
    ///
    /// Whether to suppress a restriction lint in constant code. In same
    /// cases the restructured operation might not be unavoidable, as the
    /// suggested counterparts are unavailable in constant code. This
    /// configuration will cause restriction lints to trigger even
    /// if no suggestion can be made.
    (suppress_restriction_lint_in_const: bool = false),
    /// Lint: MISSING_DOCS_IN_PRIVATE_ITEMS.
    ///
    /// Whether to **only** check for missing documentation in items visible within the current
    /// crate. For example, `pub(crate)` items.
    (missing_docs_in_crate_items: bool = false),
    /// Lint: LARGE_FUTURES.
    ///
    /// The maximum byte size a `Future` can have, before it triggers the `clippy::large_futures` lint
    (future_size_threshold: u64 = 16 * 1024),
    /// Lint: UNNECESSARY_BOX_RETURNS.
    ///
    /// The byte size a `T` in `Box<T>` can have, below which it triggers the `clippy::unnecessary_box` lint
    (unnecessary_box_size: u64 = 128),
    /// Lint: MODULE_INCEPTION.
    ///
    /// Whether to allow module inception if it's not public.
    (allow_private_module_inception: bool = false),
    /// Lint: MIN_IDENT_CHARS.
    ///
    /// Allowed names below the minimum allowed characters. The value `".."` can be used as part of
    /// the list to indicate, that the configured values should be appended to the default
    /// configuration of Clippy. By default, any configuration will replace the default value.
    (allowed_idents_below_min_chars: FxHashSet<String> =
        DEFAULT_ALLOWED_IDENTS_BELOW_MIN_CHARS.iter().map(ToString::to_string).collect()),
    /// Lint: MIN_IDENT_CHARS.
    ///
    /// Minimum chars an ident can have, anything below or equal to this will be linted.
    (min_ident_chars_threshold: u64 = 1),
    /// Lint: UNDOCUMENTED_UNSAFE_BLOCKS.
    ///
    /// Whether to accept a safety comment to be placed above the statement containing the `unsafe` block
    (accept_comment_above_statement: bool = true),
    /// Lint: UNDOCUMENTED_UNSAFE_BLOCKS.
    ///
    /// Whether to accept a safety comment to be placed above the attributes for the `unsafe` block
    (accept_comment_above_attributes: bool = true),
    /// Lint: UNNECESSARY_RAW_STRING_HASHES.
    ///
    /// Whether to allow `r#""#` when `r""` can be used
    (allow_one_hash_in_raw_strings: bool = false),
    /// Lint: ABSOLUTE_PATHS.
    ///
    /// The maximum number of segments a path can have before being linted, anything above this will
    /// be linted.
    (absolute_paths_max_segments: u64 = 2),
    /// Lint: ABSOLUTE_PATHS.
    ///
    /// Which crates to allow absolute paths from
    (absolute_paths_allowed_crates: FxHashSet<String> = FxHashSet::default()),
    /// Lint: PATH_ENDS_WITH_EXT.
    ///
    /// Additional dotfiles (files or directories starting with a dot) to allow
    (allowed_dotfiles: FxHashSet<String> = FxHashSet::default()),
    /// Lint: EXPLICIT_ITER_LOOP
    ///
    /// Whether to recommend using implicit into iter for reborrowed values.
    ///
    /// #### Example
    /// ```no_run
    /// let mut vec = vec![1, 2, 3];
    /// let rmvec = &mut vec;
    /// for _ in rmvec.iter() {}
    /// for _ in rmvec.iter_mut() {}
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut vec = vec![1, 2, 3];
    /// let rmvec = &mut vec;
    /// for _ in &*rmvec {}
    /// for _ in &mut *rmvec {}
    /// ```
    (enforce_iter_loop_reborrow: bool = false),
    /// Lint: MISSING_SAFETY_DOC, UNNECESSARY_SAFETY_DOC, MISSING_PANICS_DOC, MISSING_ERRORS_DOC
    ///
    /// Whether to also run the listed lints on private items.
    (check_private_items: bool = false),
}

/// Search for the configuration file.
///
/// # Errors
///
/// Returns any unexpected filesystem error encountered when searching for the config file
pub fn lookup_conf_file() -> io::Result<(Option<PathBuf>, Vec<String>)> {
    /// Possible filename to search for.
    const CONFIG_FILE_NAMES: [&str; 2] = [".clippy.toml", "clippy.toml"];

    // Start looking for a config file in CLIPPY_CONF_DIR, or failing that, CARGO_MANIFEST_DIR.
    // If neither of those exist, use ".". (Update documentation if this priority changes)
    let mut current = env::var_os("CLIPPY_CONF_DIR")
        .or_else(|| env::var_os("CARGO_MANIFEST_DIR"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .canonicalize()?;

    let mut found_config: Option<PathBuf> = None;
    let mut warnings = vec![];

    loop {
        for config_file_name in &CONFIG_FILE_NAMES {
            if let Ok(config_file) = current.join(config_file_name).canonicalize() {
                match fs::metadata(&config_file) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {},
                    Err(e) => return Err(e),
                    Ok(md) if md.is_dir() => {},
                    Ok(_) => {
                        // warn if we happen to find two config files #8323
                        if let Some(ref found_config) = found_config {
                            warnings.push(format!(
                                "using config file `{}`, `{}` will be ignored",
                                found_config.display(),
                                config_file.display()
                            ));
                        } else {
                            found_config = Some(config_file);
                        }
                    },
                }
            }
        }

        if found_config.is_some() {
            return Ok((found_config, warnings));
        }

        // If the current directory has no parent, we're done searching.
        if !current.pop() {
            return Ok((None, warnings));
        }
    }
}

fn deserialize(file: &SourceFile) -> TryConf {
    match toml::de::Deserializer::new(file.src.as_ref().unwrap()).deserialize_map(ConfVisitor(file)) {
        Ok(mut conf) => {
            extend_vec_if_indicator_present(&mut conf.conf.doc_valid_idents, DEFAULT_DOC_VALID_IDENTS);
            extend_vec_if_indicator_present(&mut conf.conf.disallowed_names, DEFAULT_DISALLOWED_NAMES);
            // TODO: THIS SHOULD BE TESTED, this comment will be gone soon
            if conf.conf.allowed_idents_below_min_chars.contains(&"..".to_owned()) {
                conf.conf
                    .allowed_idents_below_min_chars
                    .extend(DEFAULT_ALLOWED_IDENTS_BELOW_MIN_CHARS.iter().map(ToString::to_string));
            }

            conf
        },
        Err(e) => TryConf::from_toml_error(file, &e),
    }
}

fn extend_vec_if_indicator_present(vec: &mut Vec<String>, default: &[&str]) {
    if vec.contains(&"..".to_string()) {
        vec.extend(default.iter().map(ToString::to_string));
    }
}

impl Conf {
    pub fn read(sess: &Session, path: &io::Result<(Option<PathBuf>, Vec<String>)>) -> &'static Conf {
        static CONF: OnceLock<Conf> = OnceLock::new();
        CONF.get_or_init(|| Conf::read_inner(sess, path))
    }

    fn read_inner(sess: &Session, path: &io::Result<(Option<PathBuf>, Vec<String>)>) -> Conf {
        match path {
            Ok((_, warnings)) => {
                for warning in warnings {
                    sess.dcx().warn(warning.clone());
                }
            },
            Err(error) => {
                sess.dcx()
                    .err(format!("error finding Clippy's configuration file: {error}"));
            },
        }

        let TryConf {
            mut conf,
            errors,
            warnings,
        } = match path {
            Ok((Some(path), _)) => match sess.source_map().load_file(path) {
                Ok(file) => deserialize(&file),
                Err(error) => {
                    sess.dcx().err(format!("failed to read `{}`: {error}", path.display()));
                    TryConf::default()
                },
            },
            _ => TryConf::default(),
        };

        conf.msrv.read_cargo(sess);

        // all conf errors are non-fatal, we just use the default conf in case of error
        for error in errors {
            sess.dcx().span_err(
                error.span,
                format!("error reading Clippy's configuration file: {}", error.message),
            );
        }

        for warning in warnings {
            sess.dcx().span_warn(
                warning.span,
                format!("error reading Clippy's configuration file: {}", warning.message),
            );
        }

        conf
    }
}

const SEPARATOR_WIDTH: usize = 4;

#[derive(Debug)]
struct FieldError(String);

impl std::error::Error for FieldError {}

impl Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(&self.0)
    }
}

impl serde::de::Error for FieldError {
    fn custom<T: Display>(msg: T) -> Self {
        Self(msg.to_string())
    }

    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        // List the available fields sorted and at least one per line, more if `CLIPPY_TERMINAL_WIDTH` is
        // set and allows it.
        use fmt::Write;

        let mut expected = expected.to_vec();
        expected.sort_unstable();

        let (rows, column_widths) = calculate_dimensions(&expected);

        let mut msg = format!("unknown field `{field}`, expected one of");
        for row in 0..rows {
            writeln!(msg).unwrap();
            for (column, column_width) in column_widths.iter().copied().enumerate() {
                let index = column * rows + row;
                let field = expected.get(index).copied().unwrap_or_default();
                write!(msg, "{:SEPARATOR_WIDTH$}{field:column_width$}", " ").unwrap();
            }
        }
        Self(msg)
    }
}

fn calculate_dimensions(fields: &[&str]) -> (usize, Vec<usize>) {
    let columns = env::var("CLIPPY_TERMINAL_WIDTH")
        .ok()
        .and_then(|s| <usize as FromStr>::from_str(&s).ok())
        .map_or(1, |terminal_width| {
            let max_field_width = fields.iter().map(|field| field.len()).max().unwrap();
            cmp::max(1, terminal_width / (SEPARATOR_WIDTH + max_field_width))
        });

    let rows = (fields.len() + (columns - 1)) / columns;

    let column_widths = (0..columns)
        .map(|column| {
            if column < columns - 1 {
                (0..rows)
                    .map(|row| {
                        let index = column * rows + row;
                        let field = fields.get(index).copied().unwrap_or_default();
                        field.len()
                    })
                    .max()
                    .unwrap()
            } else {
                // Avoid adding extra space to the last column.
                0
            }
        })
        .collect::<Vec<_>>();

    (rows, column_widths)
}

#[cfg(test)]
mod tests {
    use rustc_data_structures::fx::{FxHashMap, FxHashSet};
    use serde::de::IgnoredAny;
    use std::fs;
    use walkdir::WalkDir;

    #[test]
    fn configs_are_tested() {
        let mut names: FxHashSet<String> = crate::get_configuration_metadata()
            .into_iter()
            .map(|meta| meta.name.replace('_', "-"))
            .collect();

        let toml_files = WalkDir::new("../tests")
            .into_iter()
            .map(Result::unwrap)
            .filter(|entry| entry.file_name() == "clippy.toml");

        for entry in toml_files {
            let file = fs::read_to_string(entry.path()).unwrap();
            #[allow(clippy::zero_sized_map_values)]
            if let Ok(map) = toml::from_str::<FxHashMap<String, IgnoredAny>>(&file) {
                for name in map.keys() {
                    names.remove(name.as_str());
                }
            }
        }

        assert!(
            names.remove("allow-one-hash-in-raw-strings"),
            "remove this when #11481 is fixed"
        );

        assert!(
            names.is_empty(),
            "Configuration variable lacks test: {names:?}\nAdd a test to `tests/ui-toml`"
        );
    }
}
