Adds the `collapsible_tuple_let` lint (style, warn-by-default).

Detects `let (a, b) = { let x = expr; (x, expr2) };` where the block
is unnecessary — the expressions are fully separable and can be written
as individual `let` statements.

Closes #16750

### What the lint catches

```rust
// Before
let (var1, var2) = {
    let v1 = a_complex_expression(&mut inputs)?;
    (v1, another_complex_expression(inputs))
};

// After (MachineApplicable suggestion)
let var1 = a_complex_expression(&mut inputs)?;
let var2 = another_complex_expression(inputs);
```

### Conservative conditions (no false positives)

The lint is skipped when:
- Block-locals appear in reversed / non-declaration order in the tuple
  (would reorder side effects)
- A block-local is used in another block statement's initializer
- An inline expression precedes a block-local reference in the tuple
- The same block-local is referenced twice in the tuple
- The outer `let` carries a type annotation or `let-else`
- Any relevant span comes from a macro expansion

changelog: [`collapsible_tuple_let`]: Warn when a `let` tuple destructuring can be collapsed into individual `let` statements by removing an unnecessary block expression.
