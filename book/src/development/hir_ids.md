# `HirId` in Clippy

`HirId`s are identifiers for [fine-grained entities][fine_grained_entities]
in the [HIR Map][hir_map] of the high-level intermediate representation (HIR),
which is what we work with during a [LateLintPass].

`HirId`s are everywhere in `rustc`. If we open up the Clippy
codebase in a text editor and search `hir_id`,
we will get numerous search results across hundreds of files.

For instance, variables in `rustc` are identified using the `HirId`.
The variable name alone is not enough, since variables can be shadowed.

> Note: Besides `HirId`, the HIR has other identifiers such as
> `DefId`, `LocalDefId` and `BodyId`.
> Read more about [identifiers in the HIR][identifiers_in_hir].

## Working with `HirId`s in Clippy

A curious and observant reader of Clippy book might have noticed
that `hir_id`s are hiding right under our noses: the [Expr][hir_expr]
in `LateLintPass`'s [check_expr][late_lint_pass_check_expr]
method contains a `hir_id` field:

```rust
pub struct Expr<'hir> {
    pub hir_id: HirId,
    pub kind: ExprKind<'hir>,
    pub span: Span,
}
```

Alas, `HirId`s are very useful to Clippy developers. Below are some
examples for when we might need `HirId`s.

## `find_parent_node` method

One example of working with `HirId`s is the [find_parent_node] method
on `rustc_middle::hir::map::Map` that retrieves a node's parent node.
Assuming that we have a `LateContext` variable `cx` and an `expr` variable,
we could perform:

```rust
// `LateContext.tcx.hir` returns a `rustc_middle::hir::map::Map`
let map = cx.tcx.hir();
if let Some(parent_id) = map.find_parent_node(expr.hir_id) {
    println!("This node has a parent!");
}
```

## `attrs` method

Another example could be retrieving all the attributes attached to a node
with the [attrs] method on `rustc_middle::hir::map::Map`.

Assuming that we have an [Item] variable `item` via a [check_item] method,
we could do the following to retrive all attributes for further usage:

```rust
fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
    let attrs = cx.tcx.hir().attrs(item.hir_id());
    // Do something with the attributes of the `item` node
}
```

> Note: Read more on [The HIR Map][hir_map] section about how to convert
> `LocalDefId` to `HirId`, etc.

[attrs]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/hir/map/struct.Map.html#method.attrs
[check_item]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_item
[find_parent_node]: https://doc.rust-lang.org/beta/nightly-rustc/rustc_middle/hir/map/struct.Map.html#method.find_parent_node
[fine_grained_entities]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/hir/enum.Node.html
[hir_expr]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/hir/struct.Expr.html
[hir_map]: https://rustc-dev-guide.rust-lang.org/hir.html#the-hir-map
[identifiers_in_hir]: https://rustc-dev-guide.rust-lang.org/identifiers.html#in-the-hir
[Item]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/hir/struct.Item.html
[LateLintPass]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html
[late_lint_pass_check_expr]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_expr
