# Method Checks

In some scenarios we might want to check for methods when developing
a lint. There are two kinds of questions that we might be curious about:

- Invocation: Does an expression call a specific method?
- Definition: Does the type `Ty` of an expression define a method?

## Checking if an expr is calling a specific method

Suppose we have an `expr`, we can check whether it calls a specific
method, e.g. `our_fancy_method`, by performing a pattern match on
the [ExprKind] that we can access from `expr.kind`:

```rust
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::sym;

impl<'tcx> LateLintPass<'tcx> for OurFancyMethodLint {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        // Check our expr is calling a method with pattern matching
        if let hir::ExprKind::MethodCall(path, _, [_self_arg, ..]) = &expr.kind
            // Check the name of this method is `some_method`
            && path.ident.name == sym!(our_fancy_method)
            // Optionally, we can check the type of the self argument whenever necessary.
            // See "Type Checking" chapter of the Clippy book for more information.
        {
                println!("`expr` is a method call for `our_fancy_method`");
        }
    }
}
```

Take a closer look at the `ExprKind` enum variant [MethodCall] for more
information on the pattern matching.
As mentioned in [Define Lints](define_lints.md#lint-groups),
the `methods` module is full of pattern matching with `Methodcall`
in case the reader wishes to explore more.

Additionally, we use Clippy utils [sym] macro to conveniently compare
an input `our_fancy_method` into a `Symbol` since the `ident` [Ident]
in [PathSegment], which is a value of the `MethodCall` enum variant,
contains a field `name` that is defined as a `Symbol`.

## Checking if a type defines a specific method

While sometimes we want to check whether a method is being called or not,
other times we want to know if our type `Ty` defines a method.

To check if our type defines a method called `our_fancy_method`,
we will utilize the [check_impl_item] method that is available
in our beloved [LateLintPass] (for more information, refer to the
[lint pass](lint_passes.md) chapter in Clippy book).
This method provides us with an [ImplItem] struct, which represents
anything within an `impl` block.

Let us take a look at how we might check for the implementation of
`our_fancy_method` on a type:

```rust
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::return_ty;
use rustc_hir::{ImplItem, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::symbol::sym;

impl<'tcx> LateLintPass<'tcx> for MyTypeImpl {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'_>) {
        // Check if item is a method/function
        if let ImplItemKind::Fn(ref signature, _) = impl_item.kind
            // Check the method is named `our_fancy_method`
            && impl_item.ident.name == sym!(our_fancy_method)
            // We can also check it has a parameter `self`
            && signature.decl.implicit_self.has_implicit_self()
            // We can go further and even check if its return type is `String`
            && is_type_diagnostic_item(cx, return_ty(cx, impl_item.hir_id), sym::String)
        {
            println!("`our_fancy_method` is implemented!");
        }
    }
}
```

[check_impl_item]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_impl_item
[ExprKind]: https://doc.rust-lang.org/beta/nightly-rustc/rustc_hir/hir/enum.ExprKind.html
[Ident]: https://doc.rust-lang.org/beta/nightly-rustc/rustc_span/symbol/struct.Ident.html
[ImplItem]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/hir/struct.ImplItem.html
[LateLintPass]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html
[MethodCall]: https://doc.rust-lang.org/beta/nightly-rustc/rustc_hir/hir/enum.ExprKind.html#variant.MethodCall
[PathSegment]: https://doc.rust-lang.org/beta/nightly-rustc/rustc_hir/hir/struct.PathSegment.html
[sym]: https://doc.rust-lang.org/stable/nightly-rustc/clippy_utils/macro.sym.html
