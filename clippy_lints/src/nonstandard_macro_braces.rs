use std::{
    fmt,
    hash::{Hash, Hasher},
};

use clippy_utils::{diagnostics::span_lint_and_help, in_macro, is_direct_expn_of, source::snippet_opt};
use if_chain::if_chain;
use rustc_ast::ast;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;
use serde::{de, Deserialize};

declare_clippy_lint! {
    /// ### What it does
    /// Checks that common macros are used with consistent bracing.
    ///
    /// ### Why is this bad?
    /// This is mostly a consistency lint although using () or []
    /// doesn't give you a semicolon in item position, which can be unexpected.
    ///
    /// ### Example
    /// ```rust
    /// vec!{1, 2, 3};
    /// ```
    /// Use instead:
    /// ```rust
    /// vec![1, 2, 3];
    /// ```
    pub NONSTANDARD_MACRO_BRACES,
    nursery,
    "check consistent use of braces in macro"
}

const BRACES: &[(&str, &str)] = &[("(", ")"), ("{", "}"), ("[", "]")];

/// The (name, (open brace, close brace), source snippet)
type MacroInfo<'a> = (&'a str, &'a (String, String), String);

#[derive(Clone, Debug, Default)]
pub struct MacroBraces {
    macro_braces: FxHashMap<String, (String, String)>,
    done: FxHashSet<Span>,
}

impl MacroBraces {
    pub fn new(conf: &FxHashSet<MacroMatcher>) -> Self {
        let macro_braces = macro_braces(conf.clone());
        Self {
            macro_braces,
            done: FxHashSet::default(),
        }
    }
}

impl_lint_pass!(MacroBraces => [NONSTANDARD_MACRO_BRACES]);

impl EarlyLintPass for MacroBraces {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if let Some((name, braces, snip)) = is_offending_macro(cx, item.span, self) {
            let span = item.span.ctxt().outer_expn_data().call_site;
            emit_help(cx, snip, braces, name, span);
            self.done.insert(span);
        }
    }

    fn check_stmt(&mut self, cx: &EarlyContext<'_>, stmt: &ast::Stmt) {
        if let Some((name, braces, snip)) = is_offending_macro(cx, stmt.span, self) {
            let span = stmt.span.ctxt().outer_expn_data().call_site;
            emit_help(cx, snip, braces, name, span);
            self.done.insert(span);
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
        if let Some((name, braces, snip)) = is_offending_macro(cx, expr.span, self) {
            let span = expr.span.ctxt().outer_expn_data().call_site;
            emit_help(cx, snip, braces, name, span);
            self.done.insert(span);
        }
    }

    fn check_ty(&mut self, cx: &EarlyContext<'_>, ty: &ast::Ty) {
        if let Some((name, braces, snip)) = is_offending_macro(cx, ty.span, self) {
            let span = ty.span.ctxt().outer_expn_data().call_site;
            emit_help(cx, snip, braces, name, span);
            self.done.insert(span);
        }
    }
}

fn is_offending_macro<'a>(cx: &EarlyContext<'_>, span: Span, mac_braces: &'a MacroBraces) -> Option<MacroInfo<'a>> {
    if_chain! {
        if in_macro(span);
        if let Some((name, braces)) = find_matching_macro(span, &mac_braces.macro_braces);
        if let Some(snip) = snippet_opt(cx, span.ctxt().outer_expn_data().call_site);
        // we must check only invocation sites
        // https://github.com/rust-lang/rust-clippy/issues/7422
        if snip.starts_with(name);
        // make formatting consistent
        let c = snip.replace(" ", "");
        if !c.starts_with(&format!("{}!{}", name, braces.0));
        if !mac_braces.done.contains(&span.ctxt().outer_expn_data().call_site);
        then {
            Some((name, braces, snip))
        } else {
            None
        }
    }
}

fn emit_help(cx: &EarlyContext<'_>, snip: String, braces: &(String, String), name: &str, span: Span) {
    let with_space = &format!("! {}", braces.0);
    let without_space = &format!("!{}", braces.0);
    let mut help = snip;
    for b in BRACES.iter().filter(|b| b.0 != braces.0) {
        help = help.replace(b.0, &braces.0).replace(b.1, &braces.1);
        // Only `{` traditionally has space before the brace
        if braces.0 != "{" && help.contains(with_space) {
            help = help.replace(with_space, without_space);
        } else if braces.0 == "{" && help.contains(without_space) {
            help = help.replace(without_space, with_space);
        }
    }
    span_lint_and_help(
        cx,
        NONSTANDARD_MACRO_BRACES,
        span,
        &format!("use of irregular braces for `{}!` macro", name),
        Some(span),
        &format!("consider writing `{}`", help),
    );
}

fn find_matching_macro(
    span: Span,
    braces: &FxHashMap<String, (String, String)>,
) -> Option<(&String, &(String, String))> {
    braces
        .iter()
        .find(|(macro_name, _)| is_direct_expn_of(span, macro_name).is_some())
}

fn macro_braces(conf: FxHashSet<MacroMatcher>) -> FxHashMap<String, (String, String)> {
    let mut braces = vec![
        macro_matcher!(
            name: "print",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "println",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "eprint",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "eprintln",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "write",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "writeln",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "format",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "format_args",
            braces: ("(", ")"),
        ),
        macro_matcher!(
            name: "vec",
            braces: ("[", "]"),
        ),
    ]
    .into_iter()
    .collect::<FxHashMap<_, _>>();
    // We want users items to override any existing items
    for it in conf {
        braces.insert(it.name, it.braces);
    }
    braces
}

macro_rules! macro_matcher {
    (name: $name:expr, braces: ($open:expr, $close:expr) $(,)?) => {
        ($name.to_owned(), ($open.to_owned(), $close.to_owned()))
    };
}
pub(crate) use macro_matcher;

#[derive(Clone, Debug)]
pub struct MacroMatcher {
    name: String,
    braces: (String, String),
}

impl Hash for MacroMatcher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for MacroMatcher {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for MacroMatcher {}

impl<'de> Deserialize<'de> for MacroMatcher {
    fn deserialize<D>(deser: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Name,
            Brace,
        }
        struct MacVisitor;
        impl<'de> de::Visitor<'de> for MacVisitor {
            type Value = MacroMatcher;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct MacroMatcher")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut name = None;
                let mut brace: Option<&str> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        },
                        Field::Brace => {
                            if brace.is_some() {
                                return Err(de::Error::duplicate_field("brace"));
                            }
                            brace = Some(map.next_value()?);
                        },
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let brace = brace.ok_or_else(|| de::Error::missing_field("brace"))?;
                Ok(MacroMatcher {
                    name,
                    braces: BRACES
                        .iter()
                        .find(|b| b.0 == brace)
                        .map(|(o, c)| ((*o).to_owned(), (*c).to_owned()))
                        .ok_or_else(|| {
                            de::Error::custom(&format!("expected one of `(`, `{{`, `[` found `{}`", brace))
                        })?,
                })
            }
        }

        const FIELDS: &[&str] = &["name", "brace"];
        deser.deserialize_struct("MacroMatcher", FIELDS, MacVisitor)
    }
}
