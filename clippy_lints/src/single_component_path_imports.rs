use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use rustc_ast::node_id::{NodeId, NodeMap};
use rustc_ast::visit::{Visitor, walk_item, walk_path, walk_use_tree};
use rustc_ast::{Crate, Item, ItemKind, MacroDef, ModKind, Path, UseTree, UseTreeKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext as _, impl_lint_pass};
use rustc_span::edition::Edition;
use rustc_span::symbol::kw;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checking for imports with single component use path.
    ///
    /// ### Why is this bad?
    /// Import with single component use path such as `use cratename;`
    /// is not necessary, and thus should be removed.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use regex;
    ///
    /// fn main() {
    ///     regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    /// }
    /// ```
    /// Better as
    /// ```rust,ignore
    /// fn main() {
    ///     regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    /// }
    /// ```
    #[clippy::version = "1.43.0"]
    pub SINGLE_COMPONENT_PATH_IMPORTS,
    style,
    "imports with single component path are redundant"
}

impl_lint_pass!(SingleComponentPathImports => [SINGLE_COMPONENT_PATH_IMPORTS]);

#[derive(Default)]
pub struct SingleComponentPathImports {
    /// Buffer found usages to emit when visiting that item so that `#[allow]` works as expected
    found: NodeMap<Vec<SingleUse>>,
}

struct SingleUse {
    name: Symbol,
    span: Span,
    item_id: NodeId,
    can_suggest: bool,
}

impl EarlyLintPass for SingleComponentPathImports {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if cx.sess().opts.edition < Edition::Edition2018 {
            return;
        }

        self.check_mod(&krate.items, true);
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        for SingleUse { span, can_suggest, .. } in self.found.remove(&item.id).into_iter().flatten() {
            if can_suggest {
                span_lint_and_sugg(
                    cx,
                    SINGLE_COMPONENT_PATH_IMPORTS,
                    span,
                    "this import is redundant",
                    "remove it entirely",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            } else {
                span_lint_and_help(
                    cx,
                    SINGLE_COMPONENT_PATH_IMPORTS,
                    span,
                    "this import is redundant",
                    None,
                    "remove this import",
                );
            }
        }
    }
}

struct ImportUsageVisitor {
    // keep track of imports reused with `self`/`crate`/`super`, such as `self::std`
    // or `crate::log` in nested modules. Removing the `use` would be a compile error
    // (#10549, #7168)
    // ```
    // use std;
    //
    // fn main() {
    //     let _ = self::std::io::stdout();
    // }
    // ```
    imports_referenced_with_self: Vec<Symbol>,
    // how deep we are under the module currently being checked
    nested_depth: usize,
    is_crate_root: bool,
}

fn is_path_root_keyword(name: Symbol) -> bool {
    name == kw::SelfLower || name == kw::Crate || name == kw::Super
}

fn leading_root_keyword_count(segments: &[rustc_ast::PathSegment]) -> usize {
    segments
        .iter()
        .take_while(|seg| is_path_root_keyword(seg.ident.name))
        .count()
}

impl ImportUsageVisitor {
    fn new(is_crate_root: bool) -> Self {
        Self {
            imports_referenced_with_self: Vec::new(),
            nested_depth: 0,
            is_crate_root,
        }
    }

    // does this path refer to an import in the module we're checking?
    fn refers_to_checked_module(&self, segments: &[rustc_ast::PathSegment]) -> bool {
        if segments.is_empty() || !is_path_root_keyword(segments[0].ident.name) {
            return false;
        }

        let kw_count = leading_root_keyword_count(segments);
        let first = segments[0].ident.name;

        if first == kw::Crate {
            self.is_crate_root
        } else if first == kw::SelfLower {
            self.nested_depth == 0 && kw_count == 1
        } else {
            // `super::` repeated `nested_depth` times points at the checked module
            first == kw::Super && kw_count == self.nested_depth && self.nested_depth > 0
        }
    }

    fn record_from_path(&mut self, path: &Path) {
        let segments = &path.segments;
        if !self.refers_to_checked_module(segments) {
            return;
        }
        let kw_count = leading_root_keyword_count(segments);
        if let Some(seg) = segments.get(kw_count) {
            self.imports_referenced_with_self.push(seg.ident.name);
        }
    }
}

impl Visitor<'_> for ImportUsageVisitor {
    fn visit_item(&mut self, item: &Item) {
        if matches!(item.kind, ItemKind::Mod(_, _, ModKind::Loaded(..))) {
            self.nested_depth += 1;
            walk_item(self, item);
            self.nested_depth -= 1;
        } else {
            walk_item(self, item);
        }
    }

    fn visit_path(&mut self, path: &Path) {
        self.record_from_path(path);
        walk_path(self, path);
    }

    fn visit_use_tree(&mut self, use_tree: &UseTree) {
        // `use crate::{foo::bar, ...}`: name is in the nested tree, not the prefix
        let segments = &use_tree.prefix.segments;
        let kw_count = leading_root_keyword_count(segments);
        if kw_count > 0
            && segments.len() == kw_count
            && self.refers_to_checked_module(segments)
            && let UseTreeKind::Nested { items, .. } = &use_tree.kind
        {
            for tree in items {
                if let Some(seg) = tree.inner.prefix.segments.first() {
                    self.imports_referenced_with_self.push(seg.ident.name);
                }
            }
        }
        walk_use_tree(self, use_tree);
    }
}

impl SingleComponentPathImports {
    fn check_mod(&mut self, items: &[Box<Item>], is_crate_root: bool) {
        // keep track of imports reused with `self` keyword, such as `self::crypto_hash` in the example
        // below. Removing the `use crypto_hash;` would make this a compile error
        // ```
        // use crypto_hash;
        //
        // use self::crypto_hash::{Algorithm, Hasher};
        // ```
        let mut imports_reused_with_self = Vec::new();

        // keep track of single use statements such as `crypto_hash` in the example below
        // ```
        // use crypto_hash;
        // ```
        let mut single_use_usages = Vec::new();

        // keep track of macros defined in the module as we don't want it to trigger on this (#7106)
        // ```
        // macro_rules! foo { () => {} };
        // pub(crate) use foo;
        // ```
        let mut macros = Vec::new();

        for item in items {
            self.track_uses(item, &mut imports_reused_with_self, &mut single_use_usages, &mut macros);
        }

        // Only walk the module's AST in search of `self::xxx` paths when there are single
        // component imports left to lint, as the visitor recurses into every nested item.
        single_use_usages.retain(|usage| !imports_reused_with_self.contains(&usage.name));
        if single_use_usages.is_empty() {
            return;
        }

        let mut import_usage_visitor = ImportUsageVisitor::new(is_crate_root);
        for item in items {
            import_usage_visitor.visit_item(item);
        }

        for usage in single_use_usages {
            if !import_usage_visitor.imports_referenced_with_self.contains(&usage.name) {
                self.found.entry(usage.item_id).or_default().push(usage);
            }
        }
    }

    fn track_uses(
        &mut self,
        item: &Item,
        imports_reused_with_self: &mut Vec<Symbol>,
        single_use_usages: &mut Vec<SingleUse>,
        macros: &mut Vec<Symbol>,
    ) {
        if item.span.from_expansion() || item.vis.kind.is_pub() {
            return;
        }

        match &item.kind {
            ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) => {
                self.check_mod(items, false);
            },
            ItemKind::MacroDef(ident, MacroDef { macro_rules: true, .. }) => {
                macros.push(ident.name);
            },
            ItemKind::Use(use_tree) => {
                let segments = &use_tree.prefix.segments;

                // keep track of `use some_module;` usages
                if segments.len() == 1 {
                    if let UseTreeKind::Simple(None) = use_tree.kind {
                        let name = segments[0].ident.name;
                        if !macros.contains(&name) {
                            single_use_usages.push(SingleUse {
                                name,
                                span: item.span,
                                item_id: item.id,
                                can_suggest: true,
                            });
                        }
                    }
                    return;
                }

                if segments.is_empty() {
                    // keep track of `use {some_module, some_other_module};` usages
                    if let UseTreeKind::Nested { items, .. } = &use_tree.kind {
                        for tree in items {
                            let segments = &tree.inner.prefix.segments;
                            if segments.len() == 1
                                && let UseTreeKind::Simple(None) = tree.inner.kind
                            {
                                let name = segments[0].ident.name;
                                if !macros.contains(&name) {
                                    single_use_usages.push(SingleUse {
                                        name,
                                        span: tree.inner.span(),
                                        item_id: item.id,
                                        can_suggest: false,
                                    });
                                }
                            }
                        }
                    }
                }
                // keep track of `use self::some_module` usages
                // (`crate::` / `super::` in nested mods are handled by ImportUsageVisitor)
                else if segments[0].ident.name == kw::SelfLower {
                    // simple case such as `use self::module::SomeStruct`
                    if segments.len() > 1 {
                        imports_reused_with_self.push(segments[1].ident.name);
                        return;
                    }

                    // nested case such as `use self::{module1::Struct1, module2::Struct2}`
                    if let UseTreeKind::Nested { items, .. } = &use_tree.kind {
                        for tree in items {
                            let segments = &tree.inner.prefix.segments;
                            if !segments.is_empty() {
                                imports_reused_with_self.push(segments[0].ident.name);
                            }
                        }
                    }
                }
            },
            _ => {},
        }
    }
}
