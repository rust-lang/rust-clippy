use rustc_ast::{ast, attr};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::LateContext;
use rustc_middle::ty::{AdtDef, TyCtxt};
use rustc_session::Session;
use rustc_span::{sym, Span};
use std::str::FromStr;
use lazy_static::lazy_static;
use rustc_hash::FxHashSet;
use std::sync::RwLock;

use std::io::{self, Write};


use crate::source::snippet_opt;
use crate::tokenize_with_text;


pub struct EmissionState {
    emitted: RwLock<FxHashSet<String>>,
}

impl EmissionState {
    pub fn new() -> Self {
        Self {
            emitted: RwLock::new(FxHashSet::default()),
        }
    }

    pub fn has_emitted(&self, attr_name: &str) -> bool {
        let emitted = self.emitted.read().unwrap();
        emitted.contains(attr_name)
    }

    pub fn set_emitted(&self, attr_name: &str) {
        let mut emitted = self.emitted.write().unwrap();
        emitted.insert(attr_name.to_string());
    }

    pub fn reset(&self) {
        let mut emitted = self.emitted.write().unwrap();
        emitted.clear();
    }
}

lazy_static! {
    pub static ref GLOBAL_EMISSION_STATE: EmissionState = EmissionState::new();
}

/// Deprecation status of attributes known by Clippy.
pub enum DeprecationStatus {
    /// Attribute is deprecated
    Deprecated,
    /// Attribute is deprecated and was replaced by the named attribute
    Replaced(&'static str),
    None,
}

#[rustfmt::skip]
pub const BUILTIN_ATTRIBUTES: &[(&str, DeprecationStatus)] = &[
    ("author",                DeprecationStatus::None),
    ("version",               DeprecationStatus::None),
    ("cognitive_complexity",  DeprecationStatus::None),
    ("cyclomatic_complexity", DeprecationStatus::Replaced("cognitive_complexity")),
    ("dump",                  DeprecationStatus::None),
    ("msrv",                  DeprecationStatus::None),
    ("has_significant_drop",  DeprecationStatus::None),
];

pub struct LimitStack {
    stack: Vec<u64>,
}

impl Drop for LimitStack {
    fn drop(&mut self) {
        assert_eq!(self.stack.len(), 1);
    }
}

impl LimitStack {
    #[must_use]
    pub fn new(limit: u64) -> Self {
        Self { stack: vec![limit] }
    }
    pub fn limit(&self) -> u64 {
        *self.stack.last().expect("there should always be a value in the stack")
    }
    pub fn push_attrs(&mut self, sess: &Session, attrs: &[ast::Attribute], name: &'static str) {
        let stack = &mut self.stack;
        parse_attrs(sess, attrs, name, |val| stack.push(val));
    }
    pub fn pop_attrs(&mut self, sess: &Session, attrs: &[ast::Attribute], name: &'static str) {
        let stack = &mut self.stack;
        parse_attrs(sess, attrs, name, |val| assert_eq!(stack.pop(), Some(val)));
    }
}



pub fn get_attr<'a>(
    sess: &'a Session,
    attrs: &'a [ast::Attribute],
    name: &'static str,
) -> impl Iterator<Item = &'a ast::Attribute> {
    attrs.iter().filter(move |attr| {
        let attr = if let ast::AttrKind::Normal(ref normal) = attr.kind {
            &normal.item
        } else {
            return false;
        };


        let attr_segments = &attr.path.segments;
        

        if attr_segments.len() == 2 && attr_segments[0].ident.name == sym::clippy { 
            let attr_name = attr_segments[1].ident.name.as_str().to_string();
        

            BUILTIN_ATTRIBUTES
                .iter()
                .find_map(|&(builtin_name, ref deprecation_status)| {
                    if attr_segments[1].ident.name.as_str() == builtin_name {
                        Some(deprecation_status)
                    } else {
                        None
                    }
                })
                .map_or_else(
                    || {
                        
                        sess.dcx()
                            .span_err(attr_segments[1].ident.span, "usage of unknown attribute");
                        false
                    },
                    |deprecation_status| {

                        if !GLOBAL_EMISSION_STATE.has_emitted(&attr_name) {
                            let mut diag = sess
                                .dcx()
                                .struct_span_err(attr_segments[1].ident.span, "usage of deprecated attribute");
                        
                            match *deprecation_status {
                                DeprecationStatus::Deprecated => {
                                    GLOBAL_EMISSION_STATE.set_emitted(&attr_name);
                                    diag.emit();
                                    
                                    io::stderr().flush().unwrap(); // Flush stderr
                                    false
                                },
                                DeprecationStatus::Replaced(new_name) => {
                                    GLOBAL_EMISSION_STATE.set_emitted(&attr_name);
                                    diag.span_suggestion(
                                        attr_segments[1].ident.span,
                                        "consider using",
                                        new_name,
                                        Applicability::MachineApplicable,
                                    );
                                    diag.emit();
                                    
                                    io::stderr().flush().unwrap(); // Flush stderr
                                    false
                                },
                                DeprecationStatus::None => {
                                    diag.cancel();
                                    attr_segments[1].ident.name.as_str() == name
                                },
                            }
                        } else {
                            false
                        }
                    },
                    
                ) 
                   
        } else {
            false
        }
    })
}

fn parse_attrs<F: FnMut(u64)>(sess: &Session, attrs: &[ast::Attribute], name: &'static str, mut f: F) {
    for attr in get_attr(sess, attrs, name) {
        if let Some(ref value) = attr.value_str() {
            if let Ok(value) = FromStr::from_str(value.as_str()) {
                f(value);
            } else {
                sess.dcx().span_err(attr.span, "not a number");
            }
        } else {
            sess.dcx().span_err(attr.span, "bad clippy attribute");
        }
    }
}

pub fn get_unique_attr<'a>(
    sess: &'a Session,
    attrs: &'a [ast::Attribute],
    name: &'static str,
) -> Option<&'a ast::Attribute> {
    let mut unique_attr: Option<&ast::Attribute> = None;
    for attr in get_attr(sess, attrs, name) {
        if let Some(duplicate) = unique_attr {
            sess.dcx()
                .struct_span_err(attr.span, format!("`{name}` is defined multiple times"))
                .with_span_note(duplicate.span, "first definition found here")
                .emit();
        } else {
            unique_attr = Some(attr);
        }
    }
    unique_attr
}

/// Returns true if the attributes contain any of `proc_macro`,
/// `proc_macro_derive` or `proc_macro_attribute`, false otherwise
pub fn is_proc_macro(attrs: &[ast::Attribute]) -> bool {
    attrs.iter().any(rustc_ast::Attribute::is_proc_macro_attr)
}

/// Returns true if the attributes contain `#[doc(hidden)]`
pub fn is_doc_hidden(attrs: &[ast::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.has_name(sym::doc))
        .filter_map(ast::Attribute::meta_item_list)
        .any(|l| attr::list_contains_name(&l, sym::hidden))
}

pub fn has_non_exhaustive_attr(tcx: TyCtxt<'_>, adt: AdtDef<'_>) -> bool {
    adt.is_variant_list_non_exhaustive()
        || tcx.has_attr(adt.did(), sym::non_exhaustive)
        || adt.variants().iter().any(|variant_def| {
            variant_def.is_field_list_non_exhaustive() || tcx.has_attr(variant_def.def_id, sym::non_exhaustive)
        })
        || adt
            .all_fields()
            .any(|field_def| tcx.has_attr(field_def.did, sym::non_exhaustive))
}

/// Checks if the given span contains a `#[cfg(..)]` attribute
pub fn span_contains_cfg(cx: &LateContext<'_>, s: Span) -> bool {
    let Some(snip) = snippet_opt(cx, s) else {
        // Assume true. This would require either an invalid span, or one which crosses file boundaries.
        return true;
    };
    let mut iter = tokenize_with_text(&snip);

    // Search for the token sequence [`#`, `[`, `cfg`]
    while iter.any(|(t, _)| matches!(t, TokenKind::Pound)) {
        let mut iter = iter.by_ref().skip_while(|(t, _)| {
            matches!(
                t,
                TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
            )
        });
        if matches!(iter.next(), Some((TokenKind::OpenBracket, _)))
            && matches!(iter.next(), Some((TokenKind::Ident, "cfg")))
        {
            return true;
        }
    }
    false
}
