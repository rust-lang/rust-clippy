use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::IP_CONSTANT;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, func: &Expr<'_>, args: &[Expr<'_>]) {
    if let ExprKind::Path(QPath::TypeRelative(
        Ty {
            kind: TyKind::Path(QPath::Resolved(_, func_path)),
            ..
        },
        p,
    )) = func.kind
        && p.ident.as_str() == "new"
        && let Some(func_def_id) = func_path.res.opt_def_id()
        && (cx.tcx.is_diagnostic_item(sym::Ipv4Addr, func_def_id)
            || cx.tcx.is_diagnostic_item(sym::Ipv6Addr, func_def_id))
        && let Some(constant_name) = is_ipaddr_constants(cx, args)
    {
        let sugg_snip = format!(
            "{}::{}",
            snippet(cx, func_path.span, cx.tcx.def_path_str(func_def_id).as_str()),
            constant_name
        );

        span_lint_and_sugg(
            cx,
            IP_CONSTANT,
            expr.span,
            format!("use `{sugg_snip}` instead"),
            "try",
            sugg_snip,
            Applicability::MachineApplicable,
        );
    }
}

struct Node {
    children: &'static [(u128, Node)],
    constant_name: Option<&'static str>,
}

impl Node {
    const fn new(children: &'static [(u128, Node)], constant_name: Option<&'static str>) -> Self {
        Self {
            children,
            constant_name,
        }
    }

    const fn leaf(constant_name: &'static str) -> Self {
        Self {
            children: &[],
            constant_name: Some(constant_name),
        }
    }
}

// Tree structure for IP constants
#[rustfmt::skip]
static IPADDR_CONSTANTS_TREE: Node = Node::new(&[
    (127, Node::new(&[
        (0, Node::new(&[
            (0, Node::new(&[
                (1, Node::leaf("LOCALHOST")) // IPv4 127.0.0.1
            ], None))
        ], None))
    ], None)),
    (255, Node::new(&[
        (255, Node::new(&[
            (255, Node::new(&[
                (255, Node::leaf("BROADCAST")) // IPv4 255.255.255.255
            ], None))
        ], None))
    ], None)),
    (0, Node::new(&[
        (0, Node::new(&[
            (0, Node::new(&[
                (0, Node::new(&[
                    (0, Node::new(&[
                        (0, Node::new(&[
                            (0, Node::new(&[
                                (0, Node::leaf("UNSPECIFIED")), // IPv6 ::
                                (1, Node::leaf("LOCALHOST"))    // IPv6 ::1
                            ], None))
                        ], None))
                    ], None))
                ], Some("UNSPECIFIED"))) // IPv4 0.0.0.0
            ], None))
        ], None))
    ], None)),
], None);

fn is_ipaddr_constants(cx: &LateContext<'_>, args: &[Expr<'_>]) -> Option<&'static str> {
    if args.len() != 4 && args.len() != 8 {
        return None;
    }

    // Extract integer constants from arguments
    let mut constants = Vec::new();
    for arg in args {
        if let Some(Constant::Int(constant)) = ConstEvalCtxt::new(cx).eval(arg) {
            constants.push(constant);
        } else {
            return None;
        }
    }

    let mut current_node = &IPADDR_CONSTANTS_TREE;
    for value in constants {
        if let Some((_, next_node)) = current_node.children.iter().find(|(val, _)| *val == value) {
            current_node = next_node;
        } else {
            return None; // Early termination on mismatch
        }
    }

    current_node.constant_name
}
