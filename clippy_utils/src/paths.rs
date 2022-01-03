//! This module contains paths to types and functions Clippy needs to know
//! about.
//!
//! Whenever possible, please consider diagnostic items over hardcoded paths.
//! See <https://github.com/rust-lang/rust-clippy/issues/5393> for more information.

pub const ANY_TRAIT: [&str; 3] = ["core", "any", "Any"];
#[cfg(feature = "metadata-collector-lint")]
pub const APPLICABILITY: [&str; 2] = ["rustc_lint_defs", "Applicability"];
#[cfg(feature = "metadata-collector-lint")]
pub const APPLICABILITY_VALUES: [[&str; 3]; 4] = [
    ["rustc_lint_defs", "Applicability", "Unspecified"],
    ["rustc_lint_defs", "Applicability", "HasPlaceholders"],
    ["rustc_lint_defs", "Applicability", "MaybeIncorrect"],
    ["rustc_lint_defs", "Applicability", "MachineApplicable"],
];
#[cfg(feature = "metadata-collector-lint")]
pub const DIAGNOSTIC_BUILDER: [&str; 3] = ["rustc_errors", "diagnostic_builder", "DiagnosticBuilder"];
pub const ARC_PTR_EQ: [&str; 4] = ["alloc", "sync", "Arc", "ptr_eq"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const ASSERT_EQ_MACRO: [&str; 3] = ["core", "macros", "assert_eq"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const ASSERT_MACRO: [&str; 4] = ["core", "macros", "builtin", "assert"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const ASSERT_NE_MACRO: [&str; 3] = ["core", "macros", "assert_ne"];
pub const ASMUT_TRAIT: [&str; 3] = ["core", "convert", "AsMut"];
pub const ASREF_TRAIT: [&str; 3] = ["core", "convert", "AsRef"];
/// Preferably use the diagnostic item `sym::Borrow` where possible
pub const BORROW_TRAIT: [&str; 3] = ["core", "borrow", "Borrow"];
pub const BORROW_MUT_TRAIT: [&str; 3] = ["core", "borrow", "BorrowMut"];
pub const BTREEMAP_CONTAINS_KEY: [&str; 6] = ["alloc", "collections", "btree", "map", "BTreeMap", "contains_key"];
pub const BTREEMAP_ENTRY: [&str; 6] = ["alloc", "collections", "btree", "map", "entry", "Entry"];
pub const BTREEMAP_INSERT: [&str; 6] = ["alloc", "collections", "btree", "map", "BTreeMap", "insert"];
pub const CLONE_TRAIT_METHOD: [&str; 4] = ["core", "clone", "Clone", "clone"];
pub const CMP_MAX: [&str; 3] = ["core", "cmp", "max"];
pub const CMP_MIN: [&str; 3] = ["core", "cmp", "min"];
pub const COW: [&str; 3] = ["alloc", "borrow", "Cow"];
pub const CSTRING_AS_C_STR: [&str; 5] = ["std", "ffi", "c_str", "CString", "as_c_str"];
pub const DEFAULT_TRAIT_METHOD: [&str; 4] = ["core", "default", "Default", "default"];
pub const DEREF_MUT_TRAIT_METHOD: [&str; 5] = ["core", "ops", "deref", "DerefMut", "deref_mut"];
/// Preferably use the diagnostic item `sym::deref_method` where possible
pub const DEREF_TRAIT_METHOD: [&str; 5] = ["core", "ops", "deref", "Deref", "deref"];
pub const DIR_BUILDER: [&str; 3] = ["std", "fs", "DirBuilder"];
pub const DISPLAY_TRAIT: [&str; 3] = ["core", "fmt", "Display"];
pub const DOUBLE_ENDED_ITERATOR: [&str; 4] = ["core", "iter", "traits", "DoubleEndedIterator"];
pub const DROP: [&str; 3] = ["core", "mem", "drop"];
pub const DURATION: [&str; 3] = ["core", "time", "Duration"];
#[cfg(feature = "internal-lints")]
pub const EARLY_CONTEXT: [&str; 2] = ["rustc_lint", "EarlyContext"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const EPRINT_MACRO: [&str; 3] = ["std", "macros", "eprint"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const EPRINTLN_MACRO: [&str; 3] = ["std", "macros", "eprintln"];
pub const EXIT: [&str; 3] = ["std", "process", "exit"];
pub const F32_EPSILON: [&str; 4] = ["core", "f32", "<impl f32>", "EPSILON"];
pub const F64_EPSILON: [&str; 4] = ["core", "f64", "<impl f64>", "EPSILON"];
pub const FILE: [&str; 3] = ["std", "fs", "File"];
pub const FILE_TYPE: [&str; 3] = ["std", "fs", "FileType"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const FORMAT_ARGS_MACRO: [&str; 4] = ["core", "macros", "builtin", "format_args"];
pub const FROM_FROM: [&str; 4] = ["core", "convert", "From", "from"];
pub const FROM_ITERATOR: [&str; 5] = ["core", "iter", "traits", "collect", "FromIterator"];
pub const FROM_ITERATOR_METHOD: [&str; 6] = ["core", "iter", "traits", "collect", "FromIterator", "from_iter"];
pub const FROM_STR_METHOD: [&str; 5] = ["core", "str", "traits", "FromStr", "from_str"];
pub const FUTURE_FROM_GENERATOR: [&str; 3] = ["core", "future", "from_generator"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const FUTURES_IO_ASYNCREADEXT: [&str; 3] = ["futures_util", "io", "AsyncReadExt"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const FUTURES_IO_ASYNCWRITEEXT: [&str; 3] = ["futures_util", "io", "AsyncWriteExt"];
pub const HASH: [&str; 3] = ["core", "hash", "Hash"];
pub const HASHMAP_CONTAINS_KEY: [&str; 6] = ["std", "collections", "hash", "map", "HashMap", "contains_key"];
pub const HASHMAP_ENTRY: [&str; 5] = ["std", "collections", "hash", "map", "Entry"];
pub const HASHMAP_INSERT: [&str; 6] = ["std", "collections", "hash", "map", "HashMap", "insert"];
#[cfg(feature = "internal-lints")]
pub const IDENT: [&str; 3] = ["rustc_span", "symbol", "Ident"];
#[cfg(feature = "internal-lints")]
pub const IDENT_AS_STR: [&str; 4] = ["rustc_span", "symbol", "Ident", "as_str"];
pub const INDEX: [&str; 3] = ["core", "ops", "Index"];
pub const INDEX_MUT: [&str; 3] = ["core", "ops", "IndexMut"];
pub const INSERT_STR: [&str; 4] = ["alloc", "string", "String", "insert_str"];
pub const IO_READ: [&str; 3] = ["std", "io", "Read"];
pub const IO_WRITE: [&str; 3] = ["std", "io", "Write"];
pub const IPADDR_V4: [&str; 5] = ["std", "net", "ip", "IpAddr", "V4"];
pub const IPADDR_V6: [&str; 5] = ["std", "net", "ip", "IpAddr", "V6"];
pub const ITER_REPEAT: [&str; 5] = ["core", "iter", "sources", "repeat", "repeat"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const ITERTOOLS_NEXT_TUPLE: [&str; 3] = ["itertools", "Itertools", "next_tuple"];
#[cfg(feature = "internal-lints")]
pub const KW_MODULE: [&str; 3] = ["rustc_span", "symbol", "kw"];
#[cfg(feature = "internal-lints")]
pub const LATE_CONTEXT: [&str; 2] = ["rustc_lint", "LateContext"];
#[cfg(any(feature = "internal-lints", feature = "metadata-collector-lint"))]
pub const LINT: [&str; 2] = ["rustc_lint_defs", "Lint"];
pub const MEM_DISCRIMINANT: [&str; 3] = ["core", "mem", "discriminant"];
pub const MEM_FORGET: [&str; 3] = ["core", "mem", "forget"];
pub const MEM_MANUALLY_DROP: [&str; 4] = ["core", "mem", "manually_drop", "ManuallyDrop"];
pub const MEM_MAYBEUNINIT: [&str; 4] = ["core", "mem", "maybe_uninit", "MaybeUninit"];
pub const MEM_MAYBEUNINIT_UNINIT: [&str; 5] = ["core", "mem", "maybe_uninit", "MaybeUninit", "uninit"];
pub const MEM_REPLACE: [&str; 3] = ["core", "mem", "replace"];
pub const MEM_SIZE_OF: [&str; 3] = ["core", "mem", "size_of"];
pub const MEM_SIZE_OF_VAL: [&str; 3] = ["core", "mem", "size_of_val"];
pub const MUTEX_GUARD: [&str; 4] = ["std", "sync", "mutex", "MutexGuard"];
pub const OPEN_OPTIONS: [&str; 3] = ["std", "fs", "OpenOptions"];
pub const OPS_MODULE: [&str; 2] = ["core", "ops"];
/// Preferably use the diagnostic item `sym::Option` where possible
pub const OPTION: [&str; 3] = ["core", "option", "Option"];
pub const OPTION_NONE: [&str; 4] = ["core", "option", "Option", "None"];
pub const OPTION_SOME: [&str; 4] = ["core", "option", "Option", "Some"];
pub const ORD: [&str; 3] = ["core", "cmp", "Ord"];
pub const OS_STRING_AS_OS_STR: [&str; 5] = ["std", "ffi", "os_str", "OsString", "as_os_str"];
pub const OS_STR_TO_OS_STRING: [&str; 5] = ["std", "ffi", "os_str", "OsStr", "to_os_string"];
pub const PARKING_LOT_RAWMUTEX: [&str; 3] = ["parking_lot", "raw_mutex", "RawMutex"];
pub const PARKING_LOT_RAWRWLOCK: [&str; 3] = ["parking_lot", "raw_rwlock", "RawRwLock"];
pub const PARKING_LOT_MUTEX_GUARD: [&str; 2] = ["parking_lot", "MutexGuard"];
pub const PARKING_LOT_RWLOCK_READ_GUARD: [&str; 2] = ["parking_lot", "RwLockReadGuard"];
pub const PARKING_LOT_RWLOCK_WRITE_GUARD: [&str; 2] = ["parking_lot", "RwLockWriteGuard"];
pub const PATH_BUF_AS_PATH: [&str; 4] = ["std", "path", "PathBuf", "as_path"];
pub const PATH_TO_PATH_BUF: [&str; 4] = ["std", "path", "Path", "to_path_buf"];
pub const PERMISSIONS: [&str; 3] = ["std", "fs", "Permissions"];
pub const PERMISSIONS_FROM_MODE: [&str; 6] = ["std", "os", "unix", "fs", "PermissionsExt", "from_mode"];
pub const POLL: [&str; 4] = ["core", "task", "poll", "Poll"];
pub const POLL_PENDING: [&str; 5] = ["core", "task", "poll", "Poll", "Pending"];
pub const POLL_READY: [&str; 5] = ["core", "task", "poll", "Poll", "Ready"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const PRINT_MACRO: [&str; 3] = ["std", "macros", "print"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const PRINTLN_MACRO: [&str; 3] = ["std", "macros", "println"];
pub const PTR_COPY: [&str; 3] = ["core", "intrinsics", "copy"];
pub const PTR_COPY_NONOVERLAPPING: [&str; 3] = ["core", "intrinsics", "copy_nonoverlapping"];
pub const PTR_EQ: [&str; 3] = ["core", "ptr", "eq"];
pub const PTR_SLICE_FROM_RAW_PARTS: [&str; 3] = ["core", "ptr", "slice_from_raw_parts"];
pub const PTR_SLICE_FROM_RAW_PARTS_MUT: [&str; 3] = ["core", "ptr", "slice_from_raw_parts_mut"];
pub const PTR_SWAP_NONOVERLAPPING: [&str; 3] = ["core", "ptr", "swap_nonoverlapping"];
pub const PTR_READ: [&str; 3] = ["core", "ptr", "read"];
pub const PTR_READ_UNALIGNED: [&str; 3] = ["core", "ptr", "read_unaligned"];
pub const PTR_READ_VOLATILE: [&str; 3] = ["core", "ptr", "read_volatile"];
pub const PTR_REPLACE: [&str; 3] = ["core", "ptr", "replace"];
pub const PTR_SWAP: [&str; 3] = ["core", "ptr", "swap"];
pub const PTR_WRITE: [&str; 3] = ["core", "ptr", "write"];
pub const PTR_WRITE_BYTES: [&str; 3] = ["core", "intrinsics", "write_bytes"];
pub const PTR_WRITE_UNALIGNED: [&str; 3] = ["core", "ptr", "write_unaligned"];
pub const PTR_WRITE_VOLATILE: [&str; 3] = ["core", "ptr", "write_volatile"];
pub const PUSH_STR: [&str; 4] = ["alloc", "string", "String", "push_str"];
pub const RANGE_ARGUMENT_TRAIT: [&str; 3] = ["core", "ops", "RangeBounds"];
pub const RC_PTR_EQ: [&str; 4] = ["alloc", "rc", "Rc", "ptr_eq"];
pub const REFCELL_REF: [&str; 3] = ["core", "cell", "Ref"];
pub const REFCELL_REFMUT: [&str; 3] = ["core", "cell", "RefMut"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_BUILDER_NEW: [&str; 5] = ["regex", "re_builder", "unicode", "RegexBuilder", "new"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_BYTES_BUILDER_NEW: [&str; 5] = ["regex", "re_builder", "bytes", "RegexBuilder", "new"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_BYTES_NEW: [&str; 4] = ["regex", "re_bytes", "Regex", "new"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_BYTES_SET_NEW: [&str; 5] = ["regex", "re_set", "bytes", "RegexSet", "new"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_NEW: [&str; 4] = ["regex", "re_unicode", "Regex", "new"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const REGEX_SET_NEW: [&str; 5] = ["regex", "re_set", "unicode", "RegexSet", "new"];
/// Preferably use the diagnostic item `sym::Result` where possible
pub const RESULT: [&str; 3] = ["core", "result", "Result"];
pub const RESULT_ERR: [&str; 4] = ["core", "result", "Result", "Err"];
pub const RESULT_OK: [&str; 4] = ["core", "result", "Result", "Ok"];
pub const RWLOCK_READ_GUARD: [&str; 4] = ["std", "sync", "rwlock", "RwLockReadGuard"];
pub const RWLOCK_WRITE_GUARD: [&str; 4] = ["std", "sync", "rwlock", "RwLockWriteGuard"];
pub const SERDE_DESERIALIZE: [&str; 3] = ["serde", "de", "Deserialize"];
pub const SERDE_DE_VISITOR: [&str; 3] = ["serde", "de", "Visitor"];
pub const SLICE_FROM_RAW_PARTS: [&str; 4] = ["core", "slice", "raw", "from_raw_parts"];
pub const SLICE_FROM_RAW_PARTS_MUT: [&str; 4] = ["core", "slice", "raw", "from_raw_parts_mut"];
pub const SLICE_INTO_VEC: [&str; 4] = ["alloc", "slice", "<impl [T]>", "into_vec"];
pub const SLICE_ITER: [&str; 4] = ["core", "slice", "iter", "Iter"];
pub const STDERR: [&str; 4] = ["std", "io", "stdio", "stderr"];
pub const STDOUT: [&str; 4] = ["std", "io", "stdio", "stdout"];
pub const CONVERT_IDENTITY: [&str; 3] = ["core", "convert", "identity"];
pub const STD_FS_CREATE_DIR: [&str; 3] = ["std", "fs", "create_dir"];
pub const STRING_AS_MUT_STR: [&str; 4] = ["alloc", "string", "String", "as_mut_str"];
pub const STRING_AS_STR: [&str; 4] = ["alloc", "string", "String", "as_str"];
pub const STR_ENDS_WITH: [&str; 4] = ["core", "str", "<impl str>", "ends_with"];
pub const STR_FROM_UTF8: [&str; 4] = ["core", "str", "converts", "from_utf8"];
pub const STR_LEN: [&str; 4] = ["core", "str", "<impl str>", "len"];
pub const STR_STARTS_WITH: [&str; 4] = ["core", "str", "<impl str>", "starts_with"];
#[cfg(feature = "internal-lints")]
pub const SYMBOL: [&str; 3] = ["rustc_span", "symbol", "Symbol"];
#[cfg(feature = "internal-lints")]
pub const SYMBOL_AS_STR: [&str; 4] = ["rustc_span", "symbol", "Symbol", "as_str"];
#[cfg(feature = "internal-lints")]
pub const SYMBOL_INTERN: [&str; 4] = ["rustc_span", "symbol", "Symbol", "intern"];
#[cfg(feature = "internal-lints")]
pub const SYMBOL_TO_IDENT_STRING: [&str; 4] = ["rustc_span", "symbol", "Symbol", "to_ident_string"];
#[cfg(feature = "internal-lints")]
pub const SYM_MODULE: [&str; 3] = ["rustc_span", "symbol", "sym"];
#[cfg(feature = "internal-lints")]
pub const SYNTAX_CONTEXT: [&str; 3] = ["rustc_span", "hygiene", "SyntaxContext"];
pub const TO_OWNED_METHOD: [&str; 4] = ["alloc", "borrow", "ToOwned", "to_owned"];
pub const TO_STRING_METHOD: [&str; 4] = ["alloc", "string", "ToString", "to_string"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const TOKIO_IO_ASYNCREADEXT: [&str; 5] = ["tokio", "io", "util", "async_read_ext", "AsyncReadExt"];
#[allow(clippy::invalid_paths)] // internal lints do not know about all external crates
pub const TOKIO_IO_ASYNCWRITEEXT: [&str; 5] = ["tokio", "io", "util", "async_write_ext", "AsyncWriteExt"];
pub const TRY_FROM: [&str; 4] = ["core", "convert", "TryFrom", "try_from"];
pub const VEC_AS_MUT_SLICE: [&str; 4] = ["alloc", "vec", "Vec", "as_mut_slice"];
pub const VEC_AS_SLICE: [&str; 4] = ["alloc", "vec", "Vec", "as_slice"];
pub const VEC_FROM_ELEM: [&str; 3] = ["alloc", "vec", "from_elem"];
pub const VEC_NEW: [&str; 4] = ["alloc", "vec", "Vec", "new"];
pub const VEC_RESIZE: [&str; 4] = ["alloc", "vec", "Vec", "resize"];
pub const WEAK_ARC: [&str; 3] = ["alloc", "sync", "Weak"];
pub const WEAK_RC: [&str; 3] = ["alloc", "rc", "Weak"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const WRITE_MACRO: [&str; 3] = ["core", "macros", "write"];
#[allow(clippy::invalid_paths)] // `check_path` does not seem to work for macros
pub const WRITELN_MACRO: [&str; 3] = ["core", "macros", "writeln"];
pub const PTR_NON_NULL: [&str; 4] = ["core", "ptr", "non_null", "NonNull"];
