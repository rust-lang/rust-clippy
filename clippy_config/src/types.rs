use crate::de::{
    Deserialize, DeserializeOrDefault, DiagCtxt, FromDefault, Item, create_value_list_msg, find_closest_match,
};
use core::array;
use core::fmt::Display;
use core::marker::PhantomData;
use core::ops::Range;
use itertools::Itertools;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::{Applicability, Diag, EmissionGuarantee};
use rustc_hir::PrimTy;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefIdMap;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_span::source_map::Spanned;
use toml_edit as toml;

macro_rules! concat_expr {
    ($($e:expr)*) => {
        concat!($($e),*)
    }
}

macro_rules! name_or_lit {
    ($name:ident) => {
        stringify!($name)
    };
    ($name:ident $lit:literal) => {
        $lit
    };
}

macro_rules! conf_enum {
    (
        $(#[$attrs:meta])*
        $vis:vis $name:ident {$(
            $(#[$var_attrs:meta])*
            $var_name:ident $(($var_lit:literal))?,
        )*}
    ) => {
        $(#[$attrs])*
        #[derive(Clone, Copy)]
        $vis enum $name {$(
            $(#[$var_attrs])*
            $var_name,
        )*}
        impl $name {
            const NAMES: &[&'static str] = &[$(name_or_lit!($var_name $($var_lit)?)),*];
            #[allow(dead_code)]
            const COUNT: usize = {
                enum __ITEMS__ { $($var_name,)* __COUNT__ }
                __ITEMS__::__COUNT__ as usize
            };

            pub fn name(self) -> &'static str {
                Self::NAMES[self as usize]
            }
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(name_or_lit!($var_name $($var_lit)?) => Some(Self::$var_name),)*
                    _ => None,
                }
            }
        }
        impl FromDefault<$name> for $name {
            fn from_default(default: $name) -> Self {
                default
            }
            fn display_default(default: $name) -> impl Display {
                String::display_default(default.name())
            }
        }
        impl Deserialize for $name {
            fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
                let Some(s) = value.as_str() else {
                    dcx.span_err(value.span(), "expected a string");
                    return None;
                };
                let x = Self::from_str(s);
                if x.is_none() {
                    let sp = dcx.make_sp(value.span());
                    let mut diag = dcx.inner.struct_span_err(
                        sp,
                        concat_expr!("expected one of: " $("`" name_or_lit!($var_name $($var_lit)?) "`")", "*),
                    );
                    if let Some(sugg) = find_closest_match(s, Self::NAMES) {
                        diag.span_suggestion(sp, "did you mean", sugg, Applicability::MaybeIncorrect);
                    }
                    diag.note(create_value_list_msg(dcx.width, Self::NAMES));
                    diag.emit();
                }
                x
            }
        }
    };
}

/// A finite set of items which can be dynamically assigned an order.
pub trait OrderableSet<const N: usize>: Copy {
    const NAMES: &[&str];
    fn as_index(self) -> usize;
    fn validate_ordering(_dcx: &DiagCtxt<'_>, _order_by_item: &[u8; N], _spans: &[Option<Range<usize>>; N]) -> bool {
        true
    }
}

/// A dynamic order given to a finite set of items.
#[derive(Clone)]
pub struct SetOrdering<T: OrderableSet<N>, const N: usize> {
    pub order_by_item: [u8; N],
    pub groups: [Option<String>; N],
    pub intra_group_sort: [bool; N],
    _phantom: PhantomData<T>,
}
impl<T: OrderableSet<N> + Deserialize, const N: usize> SetOrdering<T, N> {
    #[inline]
    pub fn get_order(&self, value: T) -> u8 {
        self.order_by_item[value.as_index()]
    }

    #[inline]
    pub fn get_intra_group_sort(&self, value: T) -> bool {
        self.intra_group_sort[value.as_index()]
    }

    #[inline]
    pub fn enable_intra_group_sort(&mut self) {
        self.intra_group_sort.fill(true);
    }

    pub fn enable_group(&mut self, tcx: TyCtxt<'_>, sp: Span, name: &str) {
        let mut found = false;
        for (i, group) in self.groups.iter().enumerate() {
            if group.as_deref() == Some(name) {
                self.intra_group_sort[i] = true;
                found = true;
            }
        }
        if !found {
            tcx.sess.dcx().span_warn(sp, "unknown group name");
        }
    }

    fn deserialize_item(
        dcx: &DiagCtxt<'_>,
        value: &toml::Value,
        order_by_item: &mut [Option<u8>; N],
        spans: &mut [Option<Range<usize>>; N],
        order: u8,
    ) -> Option<usize> {
        let parsed = T::deserialize(dcx, Item::Value(value))?;
        let span = value.span();
        let idx = parsed.as_index();
        if order_by_item[idx].is_none() {
            order_by_item[idx] = Some(order);
            spans[idx] = span;
            Some(idx)
        } else {
            dcx.inner
                .struct_span_err(dcx.make_sp(span), "duplicate item")
                .with_span_note(dcx.make_sp(spans[idx].clone()), "previous value here")
                .emit();
            None
        }
    }
}
impl<T: OrderableSet<N> + Deserialize, const N: usize> Deserialize for SetOrdering<T, N> {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        let Item::Value(toml::Value::Array(values)) = value else {
            dcx.span_err(value.span(), "expected an array");
            return None;
        };
        let mut spans: [Option<Range<usize>>; N] = array::from_fn(|_| None);
        let mut names: [Option<String>; N] = array::from_fn(|_| None);
        let mut order_by_item: [Option<u8>; N] = [None; N];
        let mut order = 0u8;
        for value in values {
            match value {
                toml::Value::String(_) => {
                    Self::deserialize_item(dcx, value, &mut order_by_item, &mut spans, order);
                    order += 1;
                },
                // Backwards compatibility with named sets.
                toml::Value::Array(values)
                    if values.len() == 2
                        && let Some(toml::Value::String(name)) = values.get(0)
                        && let Some(toml::Value::Array(values)) = values.get(1) =>
                {
                    if !values.is_empty() {
                        for value in values {
                            if let Some(idx) = Self::deserialize_item(dcx, value, &mut order_by_item, &mut spans, order)
                            {
                                names[idx] = Some(name.value().clone());
                            }
                        }
                        order += 1;
                    }
                },
                toml::Value::Array(values) => {
                    if !values.is_empty() {
                        for value in values {
                            Self::deserialize_item(dcx, value, &mut order_by_item, &mut spans, order);
                        }
                        order += 1;
                    }
                },
                _ => {
                    dcx.span_err(value.span(), "expected an array or a string");
                },
            }
        }
        if let Some(order_by_item) = order_by_item.try_map(|x| x) {
            T::validate_ordering(dcx, &order_by_item, &spans).then_some(Self {
                order_by_item,
                groups: names,
                intra_group_sort: [false; N],
                _phantom: PhantomData,
            })
        } else {
            dcx.span_err(
                values.span(),
                format!(
                    "missing items: {}",
                    T::NAMES
                        .iter()
                        .zip(order_by_item)
                        .filter(|(_, y)| y.is_none())
                        .format_with(", ", |(&x, _), f| f(&format_args!("`{x}`"))),
                ),
            );
            None
        }
    }
}
impl<T: OrderableSet<N> + Deserialize, D, const N: usize> DeserializeOrDefault<D> for SetOrdering<T, N>
where
    Self: FromDefault<D>,
{
    fn deserialize_or_default(dcx: &DiagCtxt<'_>, value: Item<'_>, default: D) -> Self {
        Self::deserialize(dcx, value).unwrap_or_else(|| Self::from_default(default))
    }
}

pub struct Rename {
    pub path: String,
    pub rename: String,
}

impl Deserialize for Rename {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        if let Some((span, table)) = value.as_table_like() {
            deserialize_table!(dcx, table,
                path("path"): String,
                rename("rename"): String,
            );
            let Some(path) = path else {
                dcx.span_err(span.clone(), "missing required field `path`");
                return None;
            };
            let Some(rename) = rename else {
                dcx.span_err(span.clone(), "missing required field `rename`");
                return None;
            };
            Some(Rename { path, rename })
        } else {
            dcx.span_err(value.span(), "expected an inline table");
            None
        }
    }
}

pub struct DisallowedPath {
    pub path: Spanned<String>,
    pub reason: Option<String>,
    pub allow_invalid: bool,
}
impl DisallowedPath {
    pub fn add_diagnostic(&'static self, diag: &mut Diag<'_, impl EmissionGuarantee>) {
        if let Some(reason) = &self.reason {
            diag.note(&**reason);
        }
        diag.span_note_once(self.path.span, "disallowed due to config");
    }
}
impl Deserialize for DisallowedPath {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        if let Some(s) = value.as_str() {
            Some(DisallowedPath {
                path: Spanned {
                    node: s.into(),
                    span: dcx.make_sp(value.span()),
                },
                reason: None,
                allow_invalid: false,
            })
        } else if let Some((span, table)) = value.as_table_like() {
            deserialize_table!(dcx, table,
                path("path"): Spanned<String>,
                reason("reason"): String,
                allow_invalid("allow-invalid"): bool,
            );
            let Some(path) = path else {
                dcx.span_err(span, "missing required field `path`");
                return None;
            };
            Some(DisallowedPath {
                path,
                reason,
                allow_invalid: allow_invalid.unwrap_or(false),
            })
        } else {
            dcx.span_err(value.span(), "expected either a string or an inline table");
            None
        }
    }
}

pub struct DisallowedRemappablePath {
    pub path: Spanned<String>,
    pub reason: Option<String>,
    pub replacement: Option<String>,
    pub allow_invalid: bool,
}
impl DisallowedRemappablePath {
    pub fn add_diagnostic(&'static self, sp: Span, diag: &mut Diag<'_, impl EmissionGuarantee>) {
        if let Some(replacement) = &self.replacement {
            diag.span_suggestion(
                sp,
                self.reason.as_deref().unwrap_or("use instead"),
                &**replacement,
                Applicability::MachineApplicable,
            );
        } else if let Some(reason) = &self.reason {
            diag.note(&**reason);
        }
        diag.span_note_once(self.path.span, "disallowed due to config");
    }
}
impl Deserialize for DisallowedRemappablePath {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        if let Some(s) = value.as_str() {
            Some(DisallowedRemappablePath {
                path: Spanned {
                    node: s.into(),
                    span: dcx.make_sp(value.span()),
                },
                reason: None,
                replacement: None,
                allow_invalid: false,
            })
        } else if let Some((span, table)) = value.as_table_like() {
            deserialize_table!(dcx, table,
                path("path"): Spanned<String>,
                reason("reason"): String,
                replacement("replacement"): String,
                allow_invalid("allow-invalid"): bool,
            );
            let Some(path) = path else {
                dcx.span_err(span, "missing required field `path`");
                return None;
            };
            Some(DisallowedRemappablePath {
                path,
                reason,
                replacement,
                allow_invalid: allow_invalid.unwrap_or(false),
            })
        } else {
            dcx.span_err(value.span(), "expected either a string or an inline table");
            None
        }
    }
}

pub trait DisallowedPathLike {
    fn path(&self) -> &Spanned<String>;
    fn allow_invalid(&self) -> bool;
}
impl DisallowedPathLike for DisallowedPath {
    fn path(&self) -> &Spanned<String> {
        &self.path
    }
    fn allow_invalid(&self) -> bool {
        self.allow_invalid
    }
}
impl DisallowedPathLike for DisallowedRemappablePath {
    fn path(&self) -> &Spanned<String> {
        &self.path
    }
    fn allow_invalid(&self) -> bool {
        self.allow_invalid
    }
}

fn resolve_disallowed_path(
    tcx: TyCtxt<'_>,
    path: &'static Spanned<String>,
    resolve: fn(TyCtxt<'_>, &[&str]) -> Vec<Res>,
    allowed_def_kinds: &[DefKind],
    allowed_desc: &str,
    allow_prim_tys: bool,
    allow_invalid: bool,
) -> Vec<Res> {
    let mut resolutions = resolve(tcx, &path.node.split("::").collect::<Vec<_>>());
    let mut found_def_id = None;
    let mut found_prim_ty = false;
    resolutions.retain(|res| match res {
        Res::Def(def_kind, def_id) => {
            found_def_id = Some(*def_id);
            allowed_def_kinds.contains(def_kind)
        },
        Res::PrimTy(_) => {
            found_prim_ty = true;
            allow_prim_tys
        },
        _ => false,
    });

    if resolutions.is_empty() {
        if let Some(def_id) = found_def_id {
            tcx.sess.dcx().span_warn(
                path.span,
                format!(
                    "expected a {allowed_desc}, found {} {}",
                    tcx.def_descr_article(def_id),
                    tcx.def_descr(def_id)
                ),
            );
        } else if found_prim_ty {
            tcx.sess
                .dcx()
                .span_warn(path.span, format!("expected a {allowed_desc}, found a primitive type",));
        } else if !allow_invalid {
            tcx.sess.dcx().span_warn(
                path.span,
                format!("`{}` does not refer to an existing {allowed_desc}", path.node),
            );
        }
    }

    resolutions
}

/// Creates a map of disallowed items to the reason they were disallowed.
pub fn create_disallowed_map<T: DisallowedPathLike>(
    tcx: TyCtxt<'_>,
    disallowed_paths: &'static [T],
    // pass `def_path_res` as a function to avoid depending on `clippy_utils`
    resolve: fn(TyCtxt<'_>, &[&str]) -> Vec<Res>,
    allowed_def_kinds: &[DefKind],
    allowed_desc: &str,
    allow_prim_tys: bool,
) -> (DefIdMap<&'static T>, FxHashMap<PrimTy, &'static T>) {
    let mut def_ids: DefIdMap<&'static T> = DefIdMap::default();
    let mut prim_tys: FxHashMap<PrimTy, &'static T> = FxHashMap::default();
    for disallowed_path in disallowed_paths {
        let resolutions = resolve_disallowed_path(
            tcx,
            disallowed_path.path(),
            resolve,
            allowed_def_kinds,
            allowed_desc,
            allow_prim_tys,
            disallowed_path.allow_invalid(),
        );

        for res in resolutions {
            match res {
                Res::Def(_, def_id) => {
                    def_ids.insert(def_id, disallowed_path);
                },
                Res::PrimTy(ty) => {
                    prim_tys.insert(ty, disallowed_path);
                },
                _ => unreachable!(),
            }
        }
    }
    (def_ids, prim_tys)
}

conf_enum! {
    #[derive(PartialEq, Eq)]
    pub MatchLintBehaviour {
        AllTypes,
        WellKnownTypes,
        Never,
    }
}

enum BraceKind {
    Brace,
    Bracket,
    Paren,
}

impl Deserialize for BraceKind {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        let msg = if let Some(s) = value.as_str() {
            match s {
                "{" | "{}" => return Some(BraceKind::Brace),
                "[" | "[]" => return Some(BraceKind::Bracket),
                "(" | "()" => return Some(BraceKind::Paren),
                _ => "unknown value",
            }
        } else {
            "expected a string"
        };
        let mut diag = dcx.inner.struct_span_err(dcx.make_sp(value.span()), msg);
        diag.note("possible values: `()`, `[]`, `{}`");
        diag.emit();
        None
    }
}

pub struct MacroMatcher {
    pub name: String,
    pub braces: (char, char),
}

impl Deserialize for MacroMatcher {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        if let Some((span, table)) = value.as_table_like() {
            deserialize_table!(dcx, table,
                name("name"): String,
                brace("brace"): BraceKind,
            );
            let Some(name) = name else {
                dcx.span_err(span, "missing required field `name`");
                return None;
            };
            let Some(brace) = brace else {
                dcx.span_err(span, "missing required field `brace`");
                return None;
            };
            Some(MacroMatcher {
                name,
                braces: match brace {
                    BraceKind::Brace => ('{', '}'),
                    BraceKind::Bracket => ('[', ']'),
                    BraceKind::Paren => ('(', ')'),
                },
            })
        } else {
            dcx.span_err(value.span(), "expected an inline table");
            None
        }
    }
}

conf_enum! {
    pub PubUnderscoreFieldsBehaviour {
        PubliclyExported,
        AllPubFields,
    }
}

conf_enum! {
    /// Represents the item categories that can be ordered by the source ordering lint.
    SourceItemOrderingCategory {
        Enum("enum"),
        Impl("impl"),
        Module("module"),
        Struct("struct"),
        Trait("trait"),
    }
}

#[expect(clippy::struct_excessive_bools)]
#[derive(Clone, Copy)]
pub struct SourceItemOrdering {
    pub check_enum: bool,
    pub check_impl: bool,
    pub check_mod: bool,
    pub check_struct: bool,
    pub check_trait: bool,
}
impl Deserialize for SourceItemOrdering {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        let Item::Value(toml::Value::Array(values)) = value else {
            dcx.span_err(value.span(), "expected an array");
            return None;
        };
        let mut enabled = [false; SourceItemOrderingCategory::COUNT];
        let mut spans: [Option<Range<usize>>; SourceItemOrderingCategory::COUNT] = array::from_fn(|_| None);
        for value in values {
            let Some(cat) = SourceItemOrderingCategory::deserialize(dcx, Item::Value(value)) else {
                continue;
            };
            let span = value.span();
            if enabled[cat as usize] {
                dcx.inner
                    .struct_span_err(dcx.make_sp(span), "duplicate item")
                    .with_span_note(dcx.make_sp(spans[cat as usize].clone()), "previous value here")
                    .emit();
            } else {
                enabled[cat as usize] = true;
                spans[cat as usize] = span;
            }
        }
        Some(Self {
            check_enum: enabled[SourceItemOrderingCategory::Enum as usize],
            check_impl: enabled[SourceItemOrderingCategory::Impl as usize],
            check_mod: enabled[SourceItemOrderingCategory::Module as usize],
            check_struct: enabled[SourceItemOrderingCategory::Struct as usize],
            check_trait: enabled[SourceItemOrderingCategory::Trait as usize],
        })
    }
}
impl FromDefault<()> for SourceItemOrdering {
    fn from_default((): ()) -> Self {
        Self {
            check_enum: true,
            check_impl: true,
            check_mod: true,
            check_struct: true,
            check_trait: true,
        }
    }
    fn display_default((): ()) -> impl Display {
        r#"["enum", "impl", "module", "struct", "trait"]"#
    }
}
impl DeserializeOrDefault<()> for SourceItemOrdering {
    fn deserialize_or_default(dcx: &DiagCtxt<'_>, value: Item<'_>, default: ()) -> Self {
        Self::deserialize(dcx, value).unwrap_or_else(|| Self::from_default(default))
    }
}

conf_enum! {
    pub SourceItemOrderingModuleItemKind {
        ExternCrate("extern_crate"),
        Mod("mod"),
        ForeignMod("foreign_mod"),
        Use("use"),
        Macro("macro"),
        GlobalAsm("global_asm"),
        Static("static"),
        Const("const"),
        TyAlias("ty_alias"),
        Enum("enum"),
        Struct("struct"),
        Union("union"),
        Trait("trait"),
        TraitAlias("trait_alias"),
        Impl("impl"),
        Fn("fn"),
    }
}
pub type SourceItemOrderingModuleItemGroupings =
    SetOrdering<SourceItemOrderingModuleItemKind, { SourceItemOrderingModuleItemKind::COUNT }>;
impl OrderableSet<{ Self::COUNT }> for SourceItemOrderingModuleItemKind {
    const NAMES: &[&str] = Self::NAMES;
    #[inline]
    fn as_index(self) -> usize {
        self as usize
    }
    fn validate_ordering(
        dcx: &DiagCtxt<'_>,
        order_by_item: &[u8; Self::COUNT],
        spans: &[Option<Range<usize>>; Self::COUNT],
    ) -> bool {
        let mut is_ok = true;
        for (i, &item) in order_by_item.iter().enumerate() {
            if i != Self::Use as usize && item == order_by_item[Self::Use as usize] {
                let mut diag = dcx.inner.struct_span_err(
                    dcx.make_sp(spans[i].clone()),
                    "this category must be at a different level than `use`",
                );
                diag.note_once("grouping other items with `use` items will interfere with rustfmt");
                diag.emit();
                is_ok = false;
            }
        }
        is_ok
    }
}
impl FromDefault<()> for SourceItemOrderingModuleItemGroupings {
    fn from_default((): ()) -> Self {
        Self {
            order_by_item: [0, 0, 0, 1, 2, 3, 4, 4, 5, 5, 5, 5, 5, 5, 5, 6],
            groups: [
                Some("modules".into()),
                Some("modules".into()),
                Some("modules".into()),
                Some("use".into()),
                Some("macros".into()),
                Some("global_asm".into()),
                Some("UPPER_SNAKLE_CASE".into()),
                Some("UPPER_SNAKLE_CASE".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("PascalCase".into()),
                Some("lower_snake_case".into()),
            ],
            intra_group_sort: [false; SourceItemOrderingModuleItemKind::COUNT],
            _phantom: PhantomData,
        }
    }
    fn display_default((): ()) -> impl Display {
        r#"[["modules", ["extern_crate", "mod", "foreign_mod"]], ["use", ["use"]], ["macros", ["macro"]], ["global_asm", ["global_asm"]], ["UPPER_SNAKE_CASE", ["static", "const"]], ["PascalCase", ["ty_alias", "enum", "struct", "union", "trait", "trait_alias", "impl"]], ["lower_snake_case", ["fn"]]]"#
    }
}

conf_enum! {
    #[derive(PartialEq)]
    pub SourceItemOrderingTraitAssocItemKind {
        Const("const"),
        Fn("fn"),
        Type("type"),
    }
}
pub type SourceItemOrderingTraitAssocItemKinds =
    SetOrdering<SourceItemOrderingTraitAssocItemKind, { SourceItemOrderingTraitAssocItemKind::COUNT }>;
impl OrderableSet<{ Self::COUNT }> for SourceItemOrderingTraitAssocItemKind {
    const NAMES: &[&str] = Self::NAMES;
    #[inline]
    fn as_index(self) -> usize {
        self as usize
    }
}
impl FromDefault<()> for SourceItemOrderingTraitAssocItemKinds {
    fn from_default((): ()) -> Self {
        Self {
            order_by_item: [0, 2, 1],
            groups: array::from_fn(|_| None),
            intra_group_sort: [false; SourceItemOrderingTraitAssocItemKind::COUNT],
            _phantom: PhantomData,
        }
    }
    fn display_default((): ()) -> impl Display {
        r#"["const", "type", "fn"]"#
    }
}

pub enum SourceItemOrderingWithinModuleItemGroupings {
    All,
    None,
    Custom(Vec<Spanned<String>>),
}
impl Deserialize for SourceItemOrderingWithinModuleItemGroupings {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        match value {
            Item::Value(toml::Value::String(value)) => match &**value.value() {
                "all" => Some(Self::All),
                "none" => Some(Self::None),
                _ => {
                    dcx.span_err(value.span(), "expected: `all`, `none` or a list of category names");
                    None
                },
            },
            Item::Value(toml::Value::Array(_)) => Vec::deserialize(dcx, value).map(Self::Custom),
            _ => {
                dcx.span_err(value.span(), "expected a string or an array of strings");
                None
            },
        }
    }
}
impl FromDefault<()> for SourceItemOrderingWithinModuleItemGroupings {
    fn from_default((): ()) -> Self {
        SourceItemOrderingWithinModuleItemGroupings::None
    }
    fn display_default((): ()) -> impl Display {
        r#""none""#
    }
}
impl DeserializeOrDefault<()> for SourceItemOrderingWithinModuleItemGroupings {
    fn deserialize_or_default(dcx: &DiagCtxt<'_>, value: Item<'_>, default: ()) -> Self {
        Self::deserialize(dcx, value).unwrap_or_else(|| Self::from_default(default))
    }
}
