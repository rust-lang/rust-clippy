//! A group of attributes that can be attached to Rust code in order
//! to generate a clippy lint detecting said code automatically.

use clippy_utils::get_attr;
use rustc_ast::ast::{LitFloatType, LitKind};
use rustc_ast::walk_list;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir as hir;
use rustc_hir::intravisit::{NestedVisitorMap, Visitor};
use rustc_hir::{Block, Expr, ExprKind, Pat, PatKind, QPath, Stmt, StmtKind, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::hir::map::Map;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Generates clippy code that detects the offending pattern
    ///
    /// ### Example
    /// ```rust,ignore
    /// // ./tests/ui/my_lint.rs
    /// fn foo() {
    ///     // detect the following pattern
    ///     #[clippy::author]
    ///     if x == 42 {
    ///         // but ignore everything from here on
    ///         #![clippy::author = "ignore"]
    ///     }
    ///     ()
    /// }
    /// ```
    ///
    /// Running `TESTNAME=ui/my_lint cargo uitest` will produce
    /// a `./tests/ui/new_lint.stdout` file with the generated code:
    ///
    /// ```rust,ignore
    /// // ./tests/ui/new_lint.stdout
    /// if_chain! {
    ///     if let ExprKind::If(ref cond, ref then, None) = item.kind,
    ///     if let ExprKind::Binary(BinOp::Eq, ref left, ref right) = cond.kind,
    ///     if let ExprKind::Path(ref path) = left.kind,
    ///     if let ExprKind::Lit(ref lit) = right.kind,
    ///     if let LitKind::Int(42, _) = lit.node,
    ///     then {
    ///         // report your lint here
    ///     }
    /// }
    /// ```
    pub LINT_AUTHOR,
    internal_warn,
    "helper for writing lints"
}

declare_lint_pass!(Author => [LINT_AUTHOR]);

fn prelude() {
    println!("if_chain! {{");
}

fn done() {
    println!("    then {{");
    println!("        // report your lint here");
    println!("    }}");
    println!("}}");
}

impl<'tcx> LateLintPass<'tcx> for Author {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        if !has_attr(cx, item.hir_id()) {
            return;
        }
        prelude();
        PrintVisitor::new("item").visit_item(item);
        done();
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::ImplItem<'_>) {
        if !has_attr(cx, item.hir_id()) {
            return;
        }
        prelude();
        PrintVisitor::new("item").visit_impl_item(item);
        done();
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::TraitItem<'_>) {
        if !has_attr(cx, item.hir_id()) {
            return;
        }
        prelude();
        PrintVisitor::new("item").visit_trait_item(item);
        done();
    }

    fn check_variant(&mut self, cx: &LateContext<'tcx>, var: &'tcx hir::Variant<'_>) {
        if !has_attr(cx, var.id) {
            return;
        }
        prelude();
        let parent_hir_id = cx.tcx.hir().get_parent_node(var.id);
        PrintVisitor::new("var").visit_variant(var, &hir::Generics::empty(), parent_hir_id);
        done();
    }

    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx hir::FieldDef<'_>) {
        if !has_attr(cx, field.hir_id) {
            return;
        }
        prelude();
        PrintVisitor::new("field").visit_field_def(field);
        done();
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        if !has_attr(cx, expr.hir_id) {
            return;
        }
        prelude();
        PrintVisitor::new("expr").visit_expr(expr);
        done();
    }

    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx hir::Arm<'_>) {
        if !has_attr(cx, arm.hir_id) {
            return;
        }
        prelude();
        PrintVisitor::new("arm").visit_arm(arm);
        done();
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx hir::Stmt<'_>) {
        if !has_attr(cx, stmt.hir_id) {
            return;
        }
        match stmt.kind {
            StmtKind::Expr(e) | StmtKind::Semi(e) if has_attr(cx, e.hir_id) => return,
            _ => {},
        }
        prelude();
        PrintVisitor::new("stmt").visit_stmt(stmt);
        done();
    }

    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::ForeignItem<'_>) {
        if !has_attr(cx, item.hir_id()) {
            return;
        }
        prelude();
        PrintVisitor::new("item").visit_foreign_item(item);
        done();
    }
}

impl PrintVisitor {
    #[must_use]
    fn new(s: &'static str) -> Self {
        Self {
            ids: FxHashMap::default(),
            current: s.to_owned(),
        }
    }

    fn next(&mut self, s: &'static str) -> String {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match self.ids.entry(s) {
            // already there: start numbering from `1`
            Occupied(mut occ) => {
                let val = occ.get_mut();
                *val += 1;
                format!("{}{}", s, *val)
            },
            // not there: insert and return name as given
            Vacant(vac) => {
                vac.insert(0);
                s.to_owned()
            },
        }
    }

    fn print_qpath(&mut self, path: &QPath<'_>) {
        if let QPath::LangItem(lang_item, _) = *path {
            println!(
                "    if matches!({}, QPath::LangItem(LangItem::{:?}, _));",
                self.current, lang_item,
            );
        } else {
            print!("    if match_qpath({}, &[", self.current);
            print_path(path, &mut true);
            println!("]);");
        }
    }
}

struct PrintVisitor {
    /// Fields are the current index that needs to be appended to pattern
    /// binding names
    ids: FxHashMap<&'static str, usize>,
    /// the name that needs to be destructured
    current: String,
}

impl<'tcx> Visitor<'tcx> for PrintVisitor {
    type Map = Map<'tcx>;

    #[allow(clippy::too_many_lines)]
    fn visit_expr(&mut self, expr: &Expr<'_>) {
        print!("    if let ExprKind::");
        let current = format!("{}.kind", self.current);
        match expr.kind {
            ExprKind::Box(inner) => {
                let inner_pat = self.next("inner");
                println!("Box(ref {}) = {};", inner_pat, current);
                self.current = inner_pat;
                self.visit_expr(inner);
            },
            ExprKind::Array(elements) => {
                let elements_pat = self.next("elements");
                println!("Array(ref {}) = {};", elements_pat, current);
                println!("    if {}.len() == {};", elements_pat, elements.len());
                for (i, element) in elements.iter().enumerate() {
                    self.current = format!("{}[{}]", elements_pat, i);
                    self.visit_expr(element);
                }
            },
            ExprKind::Call(func, args) => {
                let func_pat = self.next("func");
                let args_pat = self.next("args");
                println!("Call(ref {}, ref {}) = {};", func_pat, args_pat, current);
                self.current = func_pat;
                self.visit_expr(func);
                println!("    if {}.len() == {};", args_pat, args.len());
                for (i, arg) in args.iter().enumerate() {
                    self.current = format!("{}[{}]", args_pat, i);
                    self.visit_expr(arg);
                }
            },
            ExprKind::MethodCall(_method_name, ref _generics, _args, ref _fn_span) => {
                println!(
                    "MethodCall(ref method_name, ref generics, ref args, ref fn_span) = {};",
                    current
                );
                println!("    // unimplemented: `ExprKind::MethodCall` is not further destructured at the moment");
            },
            ExprKind::Tup(elements) => {
                let elements_pat = self.next("elements");
                println!("Tup(ref {}) = {};", elements_pat, current);
                println!("    if {}.len() == {};", elements_pat, elements.len());
                for (i, element) in elements.iter().enumerate() {
                    self.current = format!("{}[{}]", elements_pat, i);
                    self.visit_expr(element);
                }
            },
            ExprKind::Binary(ref op, left, right) => {
                let op_pat = self.next("op");
                let left_pat = self.next("left");
                let right_pat = self.next("right");
                println!(
                    "Binary(ref {}, ref {}, ref {}) = {};",
                    op_pat, left_pat, right_pat, current
                );
                println!("    if BinOpKind::{:?} == {}.node;", op.node, op_pat);
                self.current = left_pat;
                self.visit_expr(left);
                self.current = right_pat;
                self.visit_expr(right);
            },
            ExprKind::Unary(ref op, inner) => {
                let inner_pat = self.next("inner");
                println!("Unary(UnOp::{:?}, ref {}) = {};", op, inner_pat, current);
                self.current = inner_pat;
                self.visit_expr(inner);
            },
            ExprKind::Lit(ref lit) => {
                let lit_pat = self.next("lit");
                println!("Lit(ref {}) = {};", lit_pat, current);
                match lit.node {
                    LitKind::Bool(val) => println!("    if let LitKind::Bool({:?}) = {}.node;", val, lit_pat),
                    LitKind::Char(c) => println!("    if let LitKind::Char({:?}) = {}.node;", c, lit_pat),
                    LitKind::Err(val) => println!("    if let LitKind::Err({}) = {}.node;", val, lit_pat),
                    LitKind::Byte(b) => println!("    if let LitKind::Byte({}) = {}.node;", b, lit_pat),
                    // FIXME: also check int type
                    LitKind::Int(i, _) => println!("    if let LitKind::Int({}, _) = {}.node;", i, lit_pat),
                    LitKind::Float(_, LitFloatType::Suffixed(_)) => println!(
                        "    if let LitKind::Float(_, LitFloatType::Suffixed(_)) = {}.node;",
                        lit_pat
                    ),
                    LitKind::Float(_, LitFloatType::Unsuffixed) => println!(
                        "    if let LitKind::Float(_, LitFloatType::Unsuffixed) = {}.node;",
                        lit_pat
                    ),
                    LitKind::ByteStr(ref vec) => {
                        let vec_pat = self.next("vec");
                        println!("    if let LitKind::ByteStr(ref {}) = {}.node;", vec_pat, lit_pat);
                        println!("    if let [{:?}] = **{};", vec, vec_pat);
                    },
                    LitKind::Str(ref text, _) => {
                        let str_pat = self.next("s");
                        println!("    if let LitKind::Str(ref {}, _) = {}.node;", str_pat, lit_pat);
                        println!("    if {}.as_str() == {:?}", str_pat, &*text.as_str());
                    },
                }
            },
            ExprKind::Cast(expr, ty) => {
                let cast_pat = self.next("expr");
                let cast_ty = self.next("cast_ty");
                let qp_label = self.next("qp");

                println!("Cast(ref {}, ref {}) = {};", cast_pat, cast_ty, current);
                if let TyKind::Path(ref qp) = ty.kind {
                    println!("    if let TyKind::Path(ref {}) = {}.kind;", qp_label, cast_ty);
                    self.current = qp_label;
                    self.print_qpath(qp);
                }
                self.current = cast_pat;
                self.visit_expr(expr);
            },
            ExprKind::Type(expr, _ty) => {
                let cast_pat = self.next("expr");
                println!("Type(ref {}, _) = {};", cast_pat, current);
                self.current = cast_pat;
                self.visit_expr(expr);
            },
            ExprKind::Loop(body, _, des, _) => {
                let body_pat = self.next("body");
                let label_pat = self.next("label");
                println!(
                    "Loop(ref {}, ref {}, LoopSource::{:?}) = {};",
                    body_pat, label_pat, des, current
                );
                self.current = body_pat;
                self.visit_block(body);
            },
            ExprKind::If(cond, then, ref opt_else) => {
                let cond_pat = self.next("cond");
                let then_pat = self.next("then");
                if let Some(else_) = *opt_else {
                    let else_pat = self.next("else_");
                    println!(
                        "If(ref {}, ref {}, Some(ref {})) = {};",
                        cond_pat, then_pat, else_pat, current
                    );
                    self.current = else_pat;
                    self.visit_expr(else_);
                } else {
                    println!("If(ref {}, ref {}, None) = {};", cond_pat, then_pat, current);
                }
                self.current = cond_pat;
                self.visit_expr(cond);
                self.current = then_pat;
                self.visit_expr(then);
            },
            ExprKind::Match(expr, arms, des) => {
                let expr_pat = self.next("expr");
                let arms_pat = self.next("arms");
                println!(
                    "Match(ref {}, ref {}, MatchSource::{:?}) = {};",
                    expr_pat, arms_pat, des, current
                );
                self.current = expr_pat;
                self.visit_expr(expr);
                println!("    if {}.len() == {};", arms_pat, arms.len());
                for (i, arm) in arms.iter().enumerate() {
                    self.current = format!("{}[{}].body", arms_pat, i);
                    self.visit_expr(arm.body);
                    if let Some(ref guard) = arm.guard {
                        let guard_pat = self.next("guard");
                        println!("    if let Some(ref {}) = {}[{}].guard;", guard_pat, arms_pat, i);
                        match guard {
                            hir::Guard::If(if_expr) => {
                                let if_expr_pat = self.next("expr");
                                println!("    if let Guard::If(ref {}) = {};", if_expr_pat, guard_pat);
                                self.current = if_expr_pat;
                                self.visit_expr(if_expr);
                            },
                            hir::Guard::IfLet(if_let_pat, if_let_expr) => {
                                let if_let_pat_pat = self.next("pat");
                                let if_let_expr_pat = self.next("expr");
                                println!(
                                    "    if let Guard::IfLet(ref {}, ref {}) = {};",
                                    if_let_pat_pat, if_let_expr_pat, guard_pat
                                );
                                self.current = if_let_expr_pat;
                                self.visit_expr(if_let_expr);
                                self.current = if_let_pat_pat;
                                self.visit_pat(if_let_pat);
                            },
                        }
                    }
                    self.current = format!("{}[{}].pat", arms_pat, i);
                    self.visit_pat(arm.pat);
                }
            },
            ExprKind::Closure(ref _capture_clause, _func, _, _, _) => {
                println!("Closure(ref capture_clause, ref func, _, _, _) = {};", current);
                println!("    // unimplemented: `ExprKind::Closure` is not further destructured at the moment");
            },
            ExprKind::Yield(sub, _) => {
                let sub_pat = self.next("sub");
                println!("Yield(ref sub) = {};", current);
                self.current = sub_pat;
                self.visit_expr(sub);
            },
            ExprKind::Block(block, _) => {
                let block_pat = self.next("block");
                println!("Block(ref {}) = {};", block_pat, current);
                self.current = block_pat;
                self.visit_block(block);
            },
            ExprKind::Assign(target, value, _) => {
                let target_pat = self.next("target");
                let value_pat = self.next("value");
                println!(
                    "Assign(ref {}, ref {}, ref _span) = {};",
                    target_pat, value_pat, current
                );
                self.current = target_pat;
                self.visit_expr(target);
                self.current = value_pat;
                self.visit_expr(value);
            },
            ExprKind::AssignOp(ref op, target, value) => {
                let op_pat = self.next("op");
                let target_pat = self.next("target");
                let value_pat = self.next("value");
                println!(
                    "AssignOp(ref {}, ref {}, ref {}) = {};",
                    op_pat, target_pat, value_pat, current
                );
                println!("    if BinOpKind::{:?} == {}.node;", op.node, op_pat);
                self.current = target_pat;
                self.visit_expr(target);
                self.current = value_pat;
                self.visit_expr(value);
            },
            ExprKind::Field(object, ref field_ident) => {
                let obj_pat = self.next("object");
                let field_name_pat = self.next("field_name");
                println!("Field(ref {}, ref {}) = {};", obj_pat, field_name_pat, current);
                println!("    if {}.as_str() == {:?}", field_name_pat, field_ident.as_str());
                self.current = obj_pat;
                self.visit_expr(object);
            },
            ExprKind::Index(object, index) => {
                let object_pat = self.next("object");
                let index_pat = self.next("index");
                println!("Index(ref {}, ref {}) = {};", object_pat, index_pat, current);
                self.current = object_pat;
                self.visit_expr(object);
                self.current = index_pat;
                self.visit_expr(index);
            },
            ExprKind::Path(ref path) => {
                let path_pat = self.next("path");
                println!("Path(ref {}) = {};", path_pat, current);
                self.current = path_pat;
                self.print_qpath(path);
            },
            ExprKind::AddrOf(kind, mutability, inner) => {
                let inner_pat = self.next("inner");
                println!(
                    "AddrOf(BorrowKind::{:?}, Mutability::{:?}, ref {}) = {};",
                    kind, mutability, inner_pat, current
                );
                self.current = inner_pat;
                self.visit_expr(inner);
            },
            ExprKind::Break(ref _destination, ref opt_value) => {
                let destination_pat = self.next("destination");
                if let Some(value) = *opt_value {
                    let value_pat = self.next("value");
                    println!("Break(ref {}, Some(ref {})) = {};", destination_pat, value_pat, current);
                    self.current = value_pat;
                    self.visit_expr(value);
                } else {
                    println!("Break(ref {}, None) = {};", destination_pat, current);
                }
                // FIXME: implement label printing
            },
            ExprKind::Continue(ref _destination) => {
                let destination_pat = self.next("destination");
                println!("Again(ref {}) = {};", destination_pat, current);
                // FIXME: implement label printing
            },
            ExprKind::Ret(ref opt_value) => {
                if let Some(value) = *opt_value {
                    let value_pat = self.next("value");
                    println!("Ret(Some(ref {})) = {};", value_pat, current);
                    self.current = value_pat;
                    self.visit_expr(value);
                } else {
                    println!("Ret(None) = {};", current);
                }
            },
            ExprKind::InlineAsm(_) => {
                println!("InlineAsm(_) = {};", current);
                println!("    // unimplemented: `ExprKind::InlineAsm` is not further destructured at the moment");
            },
            ExprKind::LlvmInlineAsm(_) => {
                println!("LlvmInlineAsm(_) = {};", current);
                println!("    // unimplemented: `ExprKind::LlvmInlineAsm` is not further destructured at the moment");
            },
            ExprKind::Struct(path, fields, ref opt_base) => {
                let path_pat = self.next("path");
                let fields_pat = self.next("fields");
                if let Some(base) = *opt_base {
                    let base_pat = self.next("base");
                    println!(
                        "Struct(ref {}, ref {}, Some(ref {})) = {};",
                        path_pat, fields_pat, base_pat, current
                    );
                    self.current = base_pat;
                    self.visit_expr(base);
                } else {
                    println!("Struct(ref {}, ref {}, None) = {};", path_pat, fields_pat, current);
                }
                self.current = path_pat;
                self.print_qpath(path);
                println!("    if {}.len() == {};", fields_pat, fields.len());
                println!("    // unimplemented: field checks");
            },
            ExprKind::ConstBlock(_) => {
                let value_pat = self.next("value");
                println!("Const({})", value_pat);
                self.current = value_pat;
            },
            // FIXME: compute length (needs type info)
            ExprKind::Repeat(value, _) => {
                let value_pat = self.next("value");
                println!("Repeat(ref {}, _) = {};", value_pat, current);
                println!("// unimplemented: repeat count check");
                self.current = value_pat;
                self.visit_expr(value);
            },
            ExprKind::Err => {
                println!("Err = {}", current);
            },
            ExprKind::DropTemps(expr) => {
                let expr_pat = self.next("expr");
                println!("DropTemps(ref {}) = {};", expr_pat, current);
                self.current = expr_pat;
                self.visit_expr(expr);
            },
        }
    }

    fn visit_block(&mut self, block: &Block<'_>) {
        println!("    if {}.stmts.len() == {};", self.current, block.stmts.len());
        let block_name = self.current.clone();
        for (i, stmt) in block.stmts.iter().enumerate() {
            self.current = format!("{}.stmts[{}]", block_name, i);
            self.visit_stmt(stmt);
        }
        if let Some(expr) = block.expr {
            self.current = self.next("trailing_expr");
            println!("    if let Some({}) = &{}.expr;", self.current, block_name);
            self.visit_expr(expr);
        } else {
            println!("    if {}.expr.is_none();", block_name);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn visit_pat(&mut self, pat: &Pat<'_>) {
        print!("    if let PatKind::");
        let current = format!("{}.kind", self.current);
        match pat.kind {
            PatKind::Wild => println!("Wild = {};", current),
            PatKind::Binding(anno, .., ident, ref sub) => {
                let anno_pat = &format!("BindingAnnotation::{:?}", anno);
                let name_pat = self.next("name");
                if let Some(sub) = *sub {
                    let sub_pat = self.next("sub");
                    println!(
                        "Binding({}, _, {}, Some(ref {})) = {};",
                        anno_pat, name_pat, sub_pat, current
                    );
                    self.current = sub_pat;
                    self.visit_pat(sub);
                } else {
                    println!("Binding({}, _, {}, None) = {};", anno_pat, name_pat, current);
                }
                println!("    if {}.as_str() == \"{}\";", name_pat, ident.as_str());
            },
            PatKind::Struct(ref path, fields, ignore) => {
                let path_pat = self.next("path");
                let fields_pat = self.next("fields");
                println!(
                    "Struct(ref {}, ref {}, {}) = {};",
                    path_pat, fields_pat, ignore, current
                );
                self.current = path_pat;
                self.print_qpath(path);
                println!("    if {}.len() == {};", fields_pat, fields.len());
                println!("    // unimplemented: field checks");
            },
            PatKind::Or(fields) => {
                let fields_pat = self.next("fields");
                println!("Or(ref {}) = {};", fields_pat, current);
                println!("    if {}.len() == {};", fields_pat, fields.len());
                println!("    // unimplemented: field checks");
            },
            PatKind::TupleStruct(ref path, fields, skip_pos) => {
                let path_pat = self.next("path");
                let fields_pat = self.next("fields");
                println!(
                    "TupleStruct(ref {}, ref {}, {:?}) = {};",
                    path_pat, fields_pat, skip_pos, current
                );
                self.current = path_pat;
                self.print_qpath(path);
                println!("    if {}.len() == {};", fields_pat, fields.len());
                println!("    // unimplemented: field checks");
            },
            PatKind::Path(ref path) => {
                let path_pat = self.next("path");
                println!("Path(ref {}) = {};", path_pat, current);
                self.current = path_pat;
                self.print_qpath(path);
            },
            PatKind::Tuple(fields, skip_pos) => {
                let fields_pat = self.next("fields");
                println!("Tuple(ref {}, {:?}) = {};", fields_pat, skip_pos, current);
                println!("    if {}.len() == {};", fields_pat, fields.len());
                println!("    // unimplemented: field checks");
            },
            PatKind::Box(pat) => {
                let pat_pat = self.next("pat");
                println!("Box(ref {}) = {};", pat_pat, current);
                self.current = pat_pat;
                self.visit_pat(pat);
            },
            PatKind::Ref(pat, muta) => {
                let pat_pat = self.next("pat");
                println!("Ref(ref {}, Mutability::{:?}) = {};", pat_pat, muta, current);
                self.current = pat_pat;
                self.visit_pat(pat);
            },
            PatKind::Lit(lit_expr) => {
                let lit_expr_pat = self.next("lit_expr");
                println!("Lit(ref {}) = {}", lit_expr_pat, current);
                self.current = lit_expr_pat;
                self.visit_expr(lit_expr);
            },
            PatKind::Range(ref start, ref end, end_kind) => {
                let start_pat = self.next("start");
                let end_pat = self.next("end");
                println!(
                    "Range(ref {}, ref {}, RangeEnd::{:?}) = {};",
                    start_pat, end_pat, end_kind, current
                );
                self.current = start_pat;
                walk_list!(self, visit_expr, start);
                self.current = end_pat;
                walk_list!(self, visit_expr, end);
            },
            PatKind::Slice(start, ref middle, end) => {
                let start_pat = self.next("start");
                let end_pat = self.next("end");
                if let Some(middle) = middle {
                    let middle_pat = self.next("middle");
                    println!(
                        "Slice(ref {}, Some(ref {}), ref {}) = {};",
                        start_pat, middle_pat, end_pat, current
                    );
                    self.current = middle_pat;
                    self.visit_pat(middle);
                } else {
                    println!("Slice(ref {}, None, ref {}) = {};", start_pat, end_pat, current);
                }
                println!("    if {}.len() == {};", start_pat, start.len());
                for (i, pat) in start.iter().enumerate() {
                    self.current = format!("{}[{}]", start_pat, i);
                    self.visit_pat(pat);
                }
                println!("    if {}.len() == {};", end_pat, end.len());
                for (i, pat) in end.iter().enumerate() {
                    self.current = format!("{}[{}]", end_pat, i);
                    self.visit_pat(pat);
                }
            },
        }
    }

    fn visit_stmt(&mut self, s: &Stmt<'_>) {
        print!("    if let StmtKind::");
        let current = format!("{}.kind", self.current);
        match s.kind {
            // A local (let) binding:
            StmtKind::Local(local) => {
                let local_pat = self.next("local");
                println!("Local(ref {}) = {};", local_pat, current);
                if let Some(init) = local.init {
                    let init_pat = self.next("init");
                    println!("    if let Some(ref {}) = {}.init;", init_pat, local_pat);
                    self.current = init_pat;
                    self.visit_expr(init);
                }
                self.current = format!("{}.pat", local_pat);
                self.visit_pat(local.pat);
            },
            // An item binding:
            StmtKind::Item(_) => {
                println!("Item(item_id) = {};", current);
            },

            // Expr without trailing semi-colon (must have unit type):
            StmtKind::Expr(e) => {
                let e_pat = self.next("e");
                println!("Expr(ref {}, _) = {}", e_pat, current);
                self.current = e_pat;
                self.visit_expr(e);
            },

            // Expr with trailing semi-colon (may have any type):
            StmtKind::Semi(e) => {
                let e_pat = self.next("e");
                println!("Semi(ref {}, _) = {}", e_pat, current);
                self.current = e_pat;
                self.visit_expr(e);
            },
        }
    }

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::None
    }
}

fn has_attr(cx: &LateContext<'_>, hir_id: hir::HirId) -> bool {
    let attrs = cx.tcx.hir().attrs(hir_id);
    get_attr(cx.sess(), attrs, "author").count() > 0
}

fn print_path(path: &QPath<'_>, first: &mut bool) {
    match *path {
        QPath::Resolved(_, path) => {
            for segment in path.segments {
                if *first {
                    *first = false;
                } else {
                    print!(", ");
                }
                print!("{:?}", segment.ident.as_str());
            }
        },
        QPath::TypeRelative(ty, segment) => match ty.kind {
            hir::TyKind::Path(ref inner_path) => {
                print_path(inner_path, first);
                if *first {
                    *first = false;
                } else {
                    print!(", ");
                }
                print!("{:?}", segment.ident.as_str());
            },
            ref other => print!("/* unimplemented: {:?}*/", other),
        },
        QPath::LangItem(..) => panic!("print_path: called for lang item qpath"),
    }
}
