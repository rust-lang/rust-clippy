use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet, snippet_indent};
use clippy_utils::str_utils::to_camel_case;
use rustc_data_structures::fx::FxIndexMap;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    Body, ExprKind, FnDecl, FnRetTy, GenericParamKind, HirId, Lifetime, LifetimeKind, LifetimeParamKind,
    LifetimeSource, LifetimeSyntax, MissingLifetimeKind, Node, Param, Pat, PatKind, QPath, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;
use rustc_span::symbol::Symbol;
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

declare_clippy_lint! {
    /// ### What it does
    /// Looks for functions that have multiple parameters of the same type
    ///
    /// ### Why is this bad?
    /// It is easy to confuse the order of the same typed parameters, resulting
    /// in possible bugs that the type system won't catch.
    /// ### Example
    /// ```no_run
    /// fn transfer(customer: String, store: String) {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct Customer(String);
    /// struct Store(String);
    ///
    /// // you can implement the Deref trait to remove the need for Store.0, etc.
    /// fn transfer(customer: Customer, store: Store) {}
    /// ```
    #[clippy::version = "1.97.0"]
    pub INTERCHANGEABLE_PARAMS,
    restriction,
    "Detects functions with multiple same type parameters, and suggests changing them
    into newtypes."
}

declare_lint_pass!(InterchangeableParams => [INTERCHANGEABLE_PARAMS]);

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
enum ArgTypeEnum {
    Path,
    Ref,
    Ptr,
    Tup,
    Other,
}

#[derive(Clone, Debug)]
struct ArgTypeData {
    argtype: ArgTypeEnum,           // type of data
    is_mut: bool,                   // mutable or not
    tyname: Option<String>,         // name of type, if any
    lifetime: Option<Lifetime>,     // lifetime of argument
    tups: Option<Vec<ArgTypeData>>, // if this has tuple data
    is_unsafe: bool,                // is the function unsafe?
    is_slice: bool,                 // is this a slice?
}
impl PartialEq for ArgTypeData {
    fn eq(&self, other: &Self) -> bool {
        self.tyname == other.tyname
    }
}
impl Eq for ArgTypeData {}

// we only want to use the type name as a hash value

impl Hash for ArgTypeData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tyname.hash(state);
    }
}

#[derive(PartialEq, Debug)]
enum FnNodeType {
    Item,
    ImplItem,
    TraitItem,
    Closure,
    Other,
}

impl Default for ArgTypeData {
    fn default() -> Self {
        Self {
            argtype: ArgTypeEnum::Other,
            is_mut: false,
            tyname: None,
            lifetime: None,
            tups: None,
            is_unsafe: false,
            is_slice: false,
        }
    }
}
#[derive(Debug)]
struct DeclString(String);

impl From<String> for DeclString {
    fn from(raw: String) -> Self {
        Self(raw)
    }
}
impl From<DeclString> for String {
    fn from(newtype: DeclString) -> String {
        newtype.0
    }
}
impl Deref for DeclString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Borrow<String> for DeclString {
    fn borrow(&self) -> &String {
        &self.0
    }
}
impl fmt::Display for DeclString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate formatting directly to the inner String's Display impl
        write!(f, "{}", self.0)
    }
}
#[derive(Debug)]
struct NewTypeString(String);
impl From<String> for NewTypeString {
    fn from(raw: String) -> Self {
        Self(raw)
    }
}
impl From<NewTypeString> for String {
    fn from(newtype: NewTypeString) -> String {
        newtype.0
    }
}
impl Deref for NewTypeString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Borrow<String> for NewTypeString {
    fn borrow(&self) -> &String {
        &self.0
    }
}
impl fmt::Display for NewTypeString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate formatting directly to the inner String's Display impl
        write!(f, "{}", self.0)
    }
}

impl<'tcx> LateLintPass<'tcx> for InterchangeableParams {
    #[allow(clippy::too_many_lines)]
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fnkind: FnKind<'tcx>,
        fndecl: &'tcx FnDecl<'tcx>,
        fnbody: &'tcx Body<'tcx>,
        span: Span,
        localdefid: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }
        let fnnode = cx.tcx.hir_node_by_def_id(localdefid);
        let is_impl = match fnnode {
            Node::Item(..) => FnNodeType::Item,
            Node::ImplItem(..) => FnNodeType::ImplItem,
            Node::TraitItem(..) => FnNodeType::TraitItem,
            Node::Expr(e) => {
                if matches!(e.kind, ExprKind::Closure(..)) {
                    FnNodeType::Closure
                } else {
                    FnNodeType::Other
                }
            },
            _ => FnNodeType::Other,
        };
        // We can't recommend this for non-item types because we can't do the newtype thing.
        if is_impl != FnNodeType::Item {
            return;
        }
        let funsafe = if let FnKind::ItemFn(_, generics, fnheader) = fnkind {
            if generics.params.is_empty() {
                fnheader.is_unsafe()
            } else {
                // we have to allow elided lifetime generics
                for gp in generics.params {
                    match &gp.kind {
                        GenericParamKind::Lifetime { kind: lt } => match lt {
                            LifetimeParamKind::Elided(MissingLifetimeKind::Ampersand) | LifetimeParamKind::Explicit => {
                            },
                            LifetimeParamKind::Elided(_) | LifetimeParamKind::Error => return,
                        },
                        _ => {
                            return;
                        },
                    }
                }
                fnheader.is_unsafe()
            }
        } else {
            false
        };

        let paramcount = fndecl.inputs.len();
        let mut types: Vec<ArgTypeData> = Vec::with_capacity(paramcount);
        let mut names: Vec<Param<'_>> = Vec::with_capacity(paramcount);
        let mut arghash: FxIndexMap<ArgTypeData, usize> = FxIndexMap::default();
        for argtype in fndecl.inputs {
            let atdopt = process_argtypes(cx, argtype, &mut arghash, funsafe);
            let Some(atd) = atdopt else {
                return;
            };
            types.push(atd.clone());
            arghash.entry(atd).and_modify(|counter| *counter += 1).or_insert(1);
        }
        for argname in fnbody.params {
            names.push(*argname);
        }
        if names.len() < 2 {
            return; // nead at least 2 parameters to handle
        }
        let paramspan = span.with_hi(fnbody.value.span.lo());

        if !arghash.clone().into_values().any(|x| x > 1) {
            return; // no duplicates
        }

        // at this point, we are ready to generate the lint text.  Right now,
        // we are only here if we have a standalone function, so we can suggest
        // newtypes.
        let mut newtypes: Vec<NewTypeString> = Vec::with_capacity(paramcount);
        let mut decls: Vec<DeclString> = Vec::with_capacity(paramcount);

        // we process the parameters here.  We've extracted the types into a structure
        // that breaks out things like tuples.  We haven't done that to the names yet, but the
        // information is there.

        for i in 0..paramcount {
            if types[i].argtype == ArgTypeEnum::Tup {
                //tups are special.  We have multiple types enclosed in parens
                // in this case, the tups field contains a list of atd structs, iterate over them.
                // There are two main cases -- the parameter is a tuple (hence the types are a tuple),
                // or name is a single token, whose type data is a tuple.
                let thistup = <ArgTypeData as Clone>::clone(&types[i]);
                let mut pnames: Vec<Pat<'_>> = Vec::new();
                let mut ptypes: Vec<DeclString> = Vec::new();
                if let PatKind::Tuple(subnames, _) = names[i].pat.kind {
                    // param names are a tuple
                    for namepat in subnames {
                        pnames.push(*namepat);
                    }
                    if let Some(ttups) = thistup.tups {
                        for j in 0..ttups.len() {
                            let (decl, newtype) = build_type_text(
                                cx,
                                <ArgTypeData as Clone>::clone(&ttups[j]),
                                pnames[j],
                                &arghash.clone(),
                            );
                            if !newtype.is_empty() {
                                newtypes.push(newtype);
                            }
                            ptypes.push(decl);
                        }
                        // construct the tuple declaration
                        let mut dnames: Vec<String> = Vec::new();
                        for name in pnames {
                            let symname = cx.tcx.hir_name(name.hir_id);
                            dnames.push(symname.to_string());
                        }
                        let symside = dnames.join(", ");
                        let typeside = ptypes
                            .iter()
                            .map(DeclString::to_string)
                            .collect::<Vec<String>>()
                            .join(", ");
                        let namedecls = DeclString(format!("({symside}): ({typeside})"));
                        decls.push(namedecls);
                    } else {
                        return;
                    }
                } else if let PatKind::Binding(bindingmode, _, nident, _) = names[i].pat.kind {
                    // param name is not a tuple, but type is
                    let paramname = format!("{}{}", bindingmode.prefix_str(), nident.as_str());
                    let mut tuptypes: Vec<String> = Vec::new();
                    if let Some(ttups) = thistup.tups {
                        // don't want to do type lookup here, single name param, tuple type
                        for thistype in &ttups {
                            let Some(tyname) = thistype.tyname.clone() else {
                                eprintln!("no name {}", line!());
                                return; // abort
                            };
                            tuptypes.push(tyname);
                        }
                        let decl = DeclString(format!("{paramname}:({})", tuptypes.join(", ")));
                        decls.push(decl);
                    } else {
                        let tupnewopt = build_type_text(
                            cx,
                            <ArgTypeData as Clone>::clone(&types[i]),
                            pnames[i],
                            &arghash.clone(),
                        );
                        let (typename, newtype) = tupnewopt;
                        let decl = DeclString(format!("{paramname}: {typename}"));
                        decls.push(decl);
                        if !newtype.is_empty() {
                            newtypes.push(newtype);
                        }
                    }
                } else {
                    return;
                }
            } else {
                // argument isn't a tuple
                let (decl, newtype) = build_type_text(
                    cx,
                    <ArgTypeData as Clone>::clone(&types[i]),
                    *names[i].pat,
                    &arghash.clone(),
                );
                if !newtype.is_empty() {
                    newtypes.push(newtype);
                }
                let symside = if matches!(names[i].pat.kind, PatKind::Wild) {
                    "_".to_string()
                } else {
                    let symname = cx.tcx.hir_name(names[i].pat.hir_id);
                    symname.as_str().to_string()
                };
                let namedecls: String = format!("{symside}: {decl}");

                decls.push(DeclString(namedecls));
            }
        }

        // fnspan is the span from the start of the function to the character before
        // the first parameter name.
        let fnspan = span.with_hi(names[0].span.lo());
        let indent = snippet_indent(cx, fnspan).unwrap_or_default();
        let fnstartsnip = snippet(cx, fnspan, "..");
        let indenttxt = format!("\n{indent}");
        let outputspan = match fndecl.output {
            FnRetTy::DefaultReturn(retspan) => retspan,
            FnRetTy::Return(ty) => ty.span,
        };
        let output = snippet(cx, outputspan, "..");
        let finalspan = outputspan.between(fnbody.value.span);
        let finalout = snippet(cx, finalspan, "..");
        if newtypes.is_empty() {
            // no newtypes created, don't signal
            return;
        }
        let ntjoin: String = newtypes
            .iter()
            .map(|item| item.0.as_str())
            .collect::<Vec<&str>>()
            .join(indenttxt.as_str());
        // in order to preserve comments and formatting, we need to replace the parameter declarations,
        // but fill in the intervening gaps with the text from the original function declaration.
        // We can start from fnsnip, which goes up to the paren.
        let mut decljoin = fnstartsnip.to_string();

        for i in 0..paramcount {
            decljoin.push_str(decls[i].as_str());
            if i < paramcount - 1 {
                let midspan = names[i].span.between(names[i + 1].span);
                let midsnippet = snippet(cx, midspan, "..");
                decljoin.push_str(&midsnippet);
            } else {
                let endspan = names[i].span.between(outputspan);
                let endsnippet = snippet(cx, endspan, "..");
                decljoin.push_str(&endsnippet);
            }
        }

        let sugg = format!("{ntjoin}\n{indent}/* .. */\n{decljoin}{output}{finalout}");

        span_lint_and_sugg(
            cx,
            INTERCHANGEABLE_PARAMS,
            paramspan,
            "multiple parameters with the same type may be confusing",
            "consider using newtypes:",
            sugg,
            Applicability::HasPlaceholders,
        );

        // things to look for
        //    "standard arguments" -- x,y,z,a,b, rhs,lhs
    }
}

fn create_prefix(cx: &LateContext<'_>, atd: &ArgTypeData) -> String {
    match atd.argtype {
        ArgTypeEnum::Ref => {
            let lifetime = handle_lifetime(cx, &atd.clone());
            if atd.is_mut {
                format!("&{lifetime}mut ")
            } else {
                format!("&{lifetime}")
            }
        },
        ArgTypeEnum::Ptr => {
            if atd.is_mut {
                "*mut ".to_string()
            } else if atd.is_unsafe {
                "*const ".to_string()
            } else {
                "*".to_string()
            }
        },
        _ => String::new(),
    }
}

fn handle_lifetime(cx: &LateContext<'_>, atd: &ArgTypeData) -> String {
    if let Some(lifetime) = atd.lifetime
        && matches!(lifetime.source, LifetimeSource::Reference)
    {
        match lifetime.syntax {
            LifetimeSyntax::Implicit => String::new(),
            LifetimeSyntax::ExplicitAnonymous => "._".to_string(),
            LifetimeSyntax::ExplicitBound => {
                if let LifetimeKind::Param(defid) = lifetime.kind {
                    format!("{} ", cx.tcx.item_name(defid))
                } else {
                    String::new()
                }
            },
        }
    } else {
        String::new()
    }
}
fn process_argtypes(
    cx: &LateContext<'_>,
    argtype: &Ty<'_>,
    arghash: &mut FxIndexMap<ArgTypeData, usize>,
    is_unsafe: bool,
) -> Option<ArgTypeData> {
    match argtype.kind {
        TyKind::Path(qpath) => {
            let nameopt = qpath_to_name(cx, qpath, argtype.hir_id);
            let name = nameopt?;
            let atd = ArgTypeData {
                argtype: ArgTypeEnum::Path,
                tyname: Some(name.to_string()),
                is_unsafe,
                ..Default::default()
            };
            Some(atd)
        },
        TyKind::Ref(lifetime, mutty) => {
            // Need to handle TyKind::Slice.
            let (name, is_slice) = match mutty.ty.kind {
                TyKind::Path(qpath) => {
                    let snippet = snippet(cx, qpath.span(), "..");
                    (snippet.to_string(), false)
                },
                TyKind::Slice(ty) => {
                    if let TyKind::Path(qpath) = ty.kind {
                        let snippet = snippet(cx, qpath.span(), "..");
                        (snippet.to_string(), true)
                    } else {
                        return None;
                    }
                },
                _ => {
                    return None;
                },
            };
            let atd = ArgTypeData {
                argtype: ArgTypeEnum::Ref,
                is_mut: mutty.mutbl.is_mut(),
                tyname: Some(name),
                lifetime: Some(*lifetime),
                is_unsafe,
                is_slice,
                ..Default::default()
            };

            Some(atd)
        },

        TyKind::Ptr(mutty) => match mutty.ty.kind {
            TyKind::Path(qpath) => {
                let nameopt = qpath_to_name(cx, qpath, argtype.hir_id);
                let name = nameopt?;
                let atd = ArgTypeData {
                    argtype: ArgTypeEnum::Ptr,
                    is_mut: mutty.mutbl.is_mut(),
                    tyname: Some(name.to_string()),
                    is_unsafe,
                    ..Default::default()
                };
                Some(atd)
            },
            TyKind::Tup(tupdata) => {
                let mut tuptypes: Vec<ArgTypeData> = Vec::new();
                for thisty in tupdata {
                    let adtopt = process_argtypes(cx, thisty, &mut *arghash, is_unsafe);
                    let adt = adtopt?;
                    tuptypes.push(adt.clone());
                }
                let atd = ArgTypeData {
                    argtype: ArgTypeEnum::Ptr,
                    is_mut: mutty.mutbl.is_mut(),
                    tups: Some(tuptypes),
                    is_unsafe,
                    ..Default::default()
                };
                Some(atd)
            },

            _ => None,
        },
        TyKind::Tup(tupdata) => {
            // tuples are a sort of recursive thing.  tupdata is an array
            // of types within the tuple.  We should walk the type tree to add the types to the
            // param hash.
            let mut tuptypes: Vec<ArgTypeData> = Vec::new();
            for thisty in tupdata {
                let adtopt = process_argtypes(cx, thisty, &mut *arghash, is_unsafe);
                let atd = adtopt?;
                tuptypes.push(atd.clone());
                // don't include tuples for now (*arghash).entry(atd).and_modify(|counter|
                // *counter += 1).or_insert(1);
            }
            let atd = ArgTypeData {
                argtype: ArgTypeEnum::Tup,
                tups: Some(tuptypes),
                is_unsafe,
                ..Default::default()
            };
            Some(atd)
        },
        _ => None,
    }
}

fn qpath_to_name(cx: &LateContext<'_>, qpath: QPath<'_>, hir_id: HirId) -> Option<Symbol> {
    let res = cx.qpath_res(&qpath, hir_id);
    match res {
        Res::PrimTy(ptype) => Some(ptype.name()),
        Res::SelfTyAlias { alias_to: defid, .. } | Res::Def(_, defid) | Res::SelfTyParam { trait_: defid, .. } => {
            Some(cx.tcx.item_name(defid))
        },
        _ => None,
    }
}

fn build_type_text(
    cx: &LateContext<'_>,
    typeatd: ArgTypeData,
    name: Pat<'_>,
    arghash: &FxIndexMap<ArgTypeData, usize>,
) -> (DeclString, NewTypeString) {
    let ptypename = match typeatd.tyname {
        Some(ref tyname) => tyname.clone(),
        None => String::new(),
    };

    // We should fiter for "standard names"
    let pnameopt = if let PatKind::Binding(bindingmode, _, ident, ..) = name.kind {
        Some(format!("{}{}", bindingmode.prefix_str(), ident.as_str()))
    } else {
        None
    };

    let camelname = if let Some(pname) = pnameopt {
        to_camel_case(pname.clone().as_str())
    } else {
        String::new()
    };
    let (decl, newtype) = if let Some(x) = arghash.get(&typeatd)
        && *x > 1
        && !ptypename.is_empty()
        && !camelname.is_empty()
        && camelname != ptypename
    {
        // duplicate type, camel_case name not matching type
        let prefix = create_prefix(cx, &typeatd.clone());
        // let suffix = create_suffix(cx, types[i]);
        let (sliceleft, sliceright) = if typeatd.is_slice { ("[", "]") } else { ("", "") };
        (
            DeclString(format!("{prefix}{sliceleft}{camelname}{sliceright}")),
            NewTypeString(format!("struct {camelname}({ptypename});")),
        )
    } else {
        let prefix = create_prefix(cx, &typeatd.clone());
        let mut ttups: Vec<String> = Vec::new();
        let tupdata = match typeatd.tups {
            Some(x) => {
                for tup in x {
                    let tuptypename = match tup.tyname {
                        Some(ref tyname) => tyname.clone(),
                        None => String::new(),
                    };
                    ttups.push(tuptypename);
                }
                format!("({})", ttups.join(", "))
            },
            None => String::new(),
        };
        let (sliceleft, sliceright) = if typeatd.is_slice { ("[", "]") } else { ("", "") };
        let decltext = format!("{prefix}{sliceleft}{ptypename}{tupdata}{sliceright}");

        (DeclString(decltext.clone()), NewTypeString(String::new()))
    };
    (decl, newtype)
}
