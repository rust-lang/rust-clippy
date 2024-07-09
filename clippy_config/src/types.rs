use crate::de::{create_value_list_msg, find_closest_match, Deserialize, DiagCtxt, FromDefault, Item};
use rustc_errors::Applicability;
use std::fmt::Display;

macro_rules! concat_expr {
    ($($e:expr)*) => {
        concat!($($e),*)
    }
}

macro_rules! conf_enum {
    (
        $(#[$attrs:meta])*
        $vis:vis $name:ident {$(
            $(#[$var_attrs:meta])*
            $var_name:ident,
        )*}
    ) => {
        $(#[$attrs])*
        #[derive(Clone, Copy)]
        $vis enum $name {$(
            $(#[$var_attrs])*
            $var_name,
        )*}
        impl $name {
            const NAMES: &[&'static str] = &[$(stringify!($var_name)),*];
            pub fn name(self) -> &'static str {
                Self::NAMES[self as usize]
            }
            #[expect(clippy::should_implement_trait)]
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(stringify!($var_name) => Some(Self::$var_name),)*
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
                    let mut diag = dcx.dcx.struct_span_err(
                        sp,
                        concat_expr!("expected one of: " $("`" stringify!($var_name) "`")", "*),
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
    pub path: String,
    pub reason: Option<String>,
}

impl Deserialize for DisallowedPath {
    fn deserialize(dcx: &DiagCtxt<'_>, value: Item<'_>) -> Option<Self> {
        if let Some(s) = value.as_str() {
            Some(DisallowedPath {
                path: s.into(),
                reason: None,
            })
        } else if let Some((span, table)) = value.as_table_like() {
            deserialize_table!(dcx, table,
                path("path"): String,
                reason("reason"): String,
            );
            let Some(path) = path else {
                dcx.span_err(span, "missing required field `path`");
                return None;
            };
            Some(DisallowedPath { path, reason })
        } else {
            dcx.span_err(value.span(), "expected either a string or an inline table");
            None
        }
    }
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
        let mut diag = dcx.dcx.struct_span_err(dcx.make_sp(value.span()), msg);
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
