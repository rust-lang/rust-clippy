use crate::utils::slice_groups_mut;
use crate::{SourceFile, Span};
use core::fmt::{self, Display};
use core::ops::{Deref, DerefMut};
use core::range::Range;
use rustc_data_structures::fx::FxHashMap;

/// The tool a lint comes from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintTool {
    Rustc,
    Clippy,
}
impl LintTool {
    /// Gets the namespace prefix to use when naming a lint including the `::`.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Rustc => "",
            Self::Clippy => "clippy::",
        }
    }

    #[must_use]
    pub fn from_prefix(s: &str) -> Option<Self> {
        (s == "clippy").then_some(Self::Clippy)
    }
}

/// The name of a lint and the tool it's from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LintName<'cx> {
    pub tool: LintTool,
    pub name: &'cx str,
}
impl<'cx> LintName<'cx> {
    #[must_use]
    pub fn new_rustc(name: &'cx str) -> Self {
        Self {
            tool: LintTool::Rustc,
            name,
        }
    }

    #[must_use]
    pub fn new_clippy(name: &'cx str) -> Self {
        Self {
            tool: LintTool::Clippy,
            name,
        }
    }
}
impl Display for LintName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tool.prefix())?;
        f.write_str(self.name)
    }
}

/// The data unique to an active lint.
#[derive(Clone, Copy)]
pub struct ActiveLintData<'cx> {
    /// The entire range of the `declare_clippy_lint` macro call.
    pub decl_range: Range<u32>,
    /// The raw text of the documentation comments. May include leading/trailing
    /// whitespace and empty lines.
    pub docs: &'cx str,
    /// The raw text of the line comments. May include leading/trailing whitespace
    /// and empty lines.
    pub group_comments: &'cx str,
    pub group: &'cx str,
    /// The raw text of the string literal including the quotation marks.
    pub desc: &'cx str,
    /// The raw text of any additional `@option` values. Starts at the comma after
    /// the description and may include trailing whitespace.
    pub opts: &'cx str,
}

/// The data unique to a deprecated lint.
#[derive(Clone, Copy)]
pub struct DeprecatedLintData<'cx> {
    pub reason: &'cx str,
}

/// The data unique to a renamed lint.
#[derive(Clone, Copy)]
pub struct RenamedLintData<'cx> {
    pub new_name: LintName<'cx>,
}

#[derive(Clone, Copy)]
pub enum LintData<'cx> {
    Active(ActiveLintData<'cx>),
    Deprecated(DeprecatedLintData<'cx>),
    Renamed(RenamedLintData<'cx>),
}

/// All the data for an active lint, including it's name.
#[derive(Clone, Copy)]
pub struct ActiveLint<'a, 'cx> {
    pub name: &'cx str,
    pub version: &'cx str,
    pub data: &'a ActiveLintData<'cx>,
}

/// Any declared lint as it's stored in the lint map. Does not include the name.
#[derive(Clone, Copy)]
pub struct Lint<'cx> {
    pub name_sp: Span<'cx>,
    pub version: &'cx str,
    pub data: LintData<'cx>,
}

/// The macro used to make a lint pass.
#[derive(Clone, Copy)]
pub enum LintPassMac {
    Declare,
    Impl,
}
impl LintPassMac {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Declare => "declare_lint_pass",
            Self::Impl => "impl_lint_pass",
        }
    }
}

pub struct LintPass<'cx> {
    /// The raw text of the documentation comments. May include leading/trailing
    /// whitespace and empty lines.
    pub docs: &'cx str,
    pub name: &'cx str,
    pub lt: Option<&'cx str>,
    pub mac: LintPassMac,
    pub decl_sp: Span<'cx>,
    pub lints: &'cx mut [&'cx str],
    pub is_early: bool,
    pub is_late: bool,
}

/// A map from a lint's name to all the other data about it.
pub struct LintMap<'cx>(pub FxHashMap<&'cx str, Lint<'cx>>);
impl<'cx> LintMap<'cx> {
    /// Creates a map from each source file to the active lints declared in that file.
    #[must_use]
    #[expect(clippy::mutable_key_type)]
    pub fn mk_by_file_map<'s>(&'s self) -> FxHashMap<&'cx SourceFile<'cx>, Vec<ActiveLint<'s, 'cx>>> {
        #[expect(clippy::default_trait_access)]
        let mut lints = FxHashMap::with_capacity_and_hasher(500, Default::default());
        for (&name, lint) in &self.0 {
            if let LintData::Active(lint_data) = &lint.data {
                lints
                    .entry(lint.name_sp.file)
                    .or_insert_with(|| Vec::with_capacity(8))
                    .push(ActiveLint {
                        name,
                        version: lint.version,
                        data: lint_data,
                    });
            }
        }
        lints
    }

    /// Iterator over all active lints declared in the given file.
    pub fn lints_in_file<'s>(&'s self, file: &SourceFile<'_>) -> impl Iterator<Item = ActiveLint<'s, 'cx>> {
        self.iter().filter_map(move |(&name, lint)| {
            if let LintData::Active(data) = &lint.data
                && lint.name_sp.file == file
            {
                Some(ActiveLint {
                    name,
                    version: lint.version,
                    data,
                })
            } else {
                None
            }
        })
    }
}
impl<'cx> Deref for LintMap<'cx> {
    type Target = FxHashMap<&'cx str, Lint<'cx>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for LintMap<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// All lint passes grouped by declaration file.
pub struct LintPasses<'cx>(pub Vec<LintPass<'cx>>);
impl<'cx> LintPasses<'cx> {
    /// Iterator over all the lint passes chuncked by the declaration file.
    pub fn iter_by_file_mut<'s>(&'s mut self) -> impl Iterator<Item = &'s mut [LintPass<'cx>]> {
        slice_groups_mut(&mut self.0, |head, tail| {
            tail.iter().take_while(|&x| x.decl_sp.file == head.decl_sp.file).count()
        })
    }

    /// Gets all the lint passes which share a file with the specified pass.
    #[must_use]
    pub fn all_in_same_file_as_mut<'s>(&'s mut self, i: usize) -> &'s mut [LintPass<'cx>] {
        let file = self[i].decl_sp.file;
        let pre = self[..i].iter().rev().take_while(|&x| x.decl_sp.file == file).count();
        let post = self[i + 1..].iter().take_while(|&x| x.decl_sp.file == file).count();
        &mut self[i - pre..i + 1 + post]
    }
}
impl<'cx> Deref for LintPasses<'cx> {
    type Target = Vec<LintPass<'cx>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for LintPasses<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ParsedLints<'cx> {
    pub lints: LintMap<'cx>,
    pub lint_passes: LintPasses<'cx>,
    pub deprecated_file: &'cx SourceFile<'cx>,
}

pub struct ConfOpt<'cx> {
    pub name: &'cx str,
    pub decl_range: Range<u32>,
    pub lints: &'cx mut [&'cx str],
    pub lints_range: Range<u32>,
}

pub struct ConfDef<'cx> {
    pub decl_sp: Span<'cx>,
    pub opts: Vec<ConfOpt<'cx>>,
}

#[derive(Clone, Copy)]
pub enum LintPassKind {
    Early,
    Late,
}
