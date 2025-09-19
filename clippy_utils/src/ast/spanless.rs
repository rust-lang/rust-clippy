use core::hash::{Hash, Hasher};
use core::mem;
use rustc_ast::token::{self, CommentKind, Delimiter, Lit, Token, TokenKind};
use rustc_ast::tokenstream::{DelimSpacing, DelimSpan, Spacing, TokenStream, TokenTree};
use rustc_ast::{
    AngleBracketedArg, AngleBracketedArgs, AnonConst, Arm, AsmMacro, AssignOpKind, AssocItemConstraint,
    AssocItemConstraintKind, AssocItemKind, AttrArgs, AttrId, AttrItem, AttrKind, AttrStyle, Attribute, BinOpKind,
    BindingMode, Block, BlockCheckMode, BorrowKind, BoundAsyncness, BoundConstness, BoundPolarity, ByRef, CaptureBy,
    Closure, ClosureBinder, Const, ConstItem, CoroutineKind, Defaultness, Delegation, DelegationMac, DelimArgs,
    EnumDef, Expr, ExprField, ExprKind, Extern, FieldDef, Fn, FnContract, FnDecl, FnHeader, FnPtrTy, FnRetTy, FnSig,
    ForLoopKind, ForeignItemKind, ForeignMod, FormatAlignment, FormatArgPosition, FormatArgPositionKind, FormatArgs,
    FormatArgsPiece, FormatArgument, FormatArgumentKind, FormatArguments, FormatCount, FormatDebugHex, FormatOptions,
    FormatPlaceholder, FormatSign, FormatTrait, GenBlockKind, GenericArg, GenericArgs, GenericBound, GenericParam,
    GenericParamKind, Generics, Impl, ImplPolarity, Inline, InlineAsm, InlineAsmOperand, InlineAsmOptions,
    InlineAsmRegOrRegClass, InlineAsmSym, InlineAsmTemplatePiece, IsAuto, Item, ItemKind, Label, Lifetime, LitKind,
    Local, LocalKind, MacCall, MacCallStmt, MacStmtStyle, MacroDef, MatchKind, MethodCall, ModKind, ModSpans,
    Movability, MutTy, Mutability, NodeId, NormalAttr, Param, ParenthesizedArgs, Pat, PatField, PatFieldsRest, PatKind,
    Path, PathSegment, PolyTraitRef, PreciseCapturingArg, QSelf, RangeEnd, RangeLimits, RangeSyntax, Recovered, Safety,
    StaticItem, Stmt, StmtKind, StrLit, StrStyle, StructExpr, StructRest, Term, Trait, TraitBoundModifiers,
    TraitImplHeader, TraitObjectSyntax, TraitRef, Ty, TyAlias, TyKind, TyPat, TyPatKind, UnOp, UnsafeBinderCastKind,
    UnsafeBinderTy, UnsafeSource, UseTree, UseTreeKind, Variant, VariantData, Visibility, VisibilityKind,
    WhereBoundPredicate, WhereClause, WhereEqPredicate, WherePredicate, WherePredicateKind, WhereRegionPredicate,
    YieldKind,
};
use rustc_data_structures::fx::FxHasher;
use rustc_span::source_map::Spanned;
use rustc_span::{ByteSymbol, ErrorGuaranteed, Ident, Span, Symbol};
use std::borrow::Cow;
use thin_vec::ThinVec;

/// Context structure for comparing/hashing AST nodes.
#[derive(Default)]
pub struct EqCtxt {
    /// Stores a mapping of a node's hash to the index it's stored at. See
    /// `UnorderedMatcher` for details.
    unordered: Vec<(u64, usize)>,
    hasher: FxHasher,
}

impl EqCtxt {
    /// Checks if two AST nodes are semantically equivalent. Small syntax differences,
    /// spans and node IDs are ignored.
    #[inline]
    #[must_use]
    pub fn eq<T: ?Sized + AstNode>(&mut self, l: &T, r: &T) -> bool {
        T::eq(self, l, r)
    }

    /// Hashes an AST node in a manner compatible with `eq`.
    #[must_use]
    pub fn hash<T: ?Sized + AstNode>(&mut self, item: &T) -> u64 {
        // Swap out the current hasher state for a fresh one. We might be in the
        // middle of hashing a node.
        let hasher = mem::take(&mut self.hasher);
        T::hash(self, item);
        let hash = self.hasher.finish();
        self.hasher = hasher;
        hash
    }

    /// Optimized version of `unordered_eq` for `PatField`. Uses the field name
    /// instead of a hash as it's almost certainly unique.
    fn eq_pat_fields(&mut self, l: &[PatField], r: &[PatField]) -> bool {
        fn hash_name(_: &mut EqCtxt, p: &PatField) -> u64 {
            u64::from(p.ident.name.as_u32())
        }

        if l.len() != r.len() {
            return false;
        }
        if l.is_empty() {
            return true;
        }

        let mut matcher = UnorderedMatcher::new(self, r, hash_name);
        matcher.eq(l, r, hash_name)
    }

    /// Compares two slices for equality ignoring the order of the elements.
    fn unordered_eq<T: AstNode>(&mut self, l: &[T], r: &[T]) -> bool {
        if l.len() != r.len() {
            return false;
        }
        if l.is_empty() {
            return true;
        }

        let mut matcher = UnorderedMatcher::new(self, r, Self::hash);
        matcher.eq(l, r, Self::hash)
    }

    fn unordered_eq_by_key<T: AstNode, U: AstNode>(&mut self, l: &[T], r: &[T], mut key: impl FnMut(&T) -> &U) -> bool {
        if l.len() != r.len() {
            return false;
        }
        if l.is_empty() {
            return true;
        }

        let mut matcher = UnorderedMatcher::new(self, r, |cx, item| cx.hash(key(item)));
        matcher.eq(l, r, |cx, item| cx.hash(key(item)))
    }

    fn write_unordered_hash<T: AstNode>(&mut self, items: &[T]) {
        self.hasher.write_length_prefix(items.len());
        let matcher = UnorderedMatcher::new(self, items, Self::hash);
        let items = &mut matcher.cx.unordered[matcher.start..];
        items.sort_unstable_by_key(|&(x, _)| x);
        for &mut (x, _) in items {
            matcher.cx.hasher.write_u64(x);
        }
    }
}

/// Helper to implement unordered slice equality. Each instance of this represent
/// a single frame on the shared stack (`EqCtxt::unordered`).
///
/// As the size of each frame isn't stored, only the top frame on the stack can be
/// safely used. Dropping this will pop the frame off the stack.
struct UnorderedMatcher<'a> {
    cx: &'a mut EqCtxt,
    start: usize,
}
impl<'a> UnorderedMatcher<'a> {
    /// Allocates a frame for the list onto the top of the stack
    fn new<'item, T>(
        cx: &'a mut EqCtxt,
        items: &'item [T],
        mut hash: impl FnMut(&mut EqCtxt, &'item T) -> u64,
    ) -> Self {
        let start = cx.unordered.len();
        cx.unordered.resize_with(start + items.len(), || (0, 0));
        for (i, item) in items.iter().enumerate() {
            let hash = hash(cx, item);
            cx.unordered[start + i] = (hash, i);
        }
        Self { cx, start }
    }

    /// Compares all items without considering order.
    ///
    /// `l` is the items that were pushed onto the stack. `r` is the list to
    /// compare against.
    fn eq<'item, T: 'item + AstNode>(
        &mut self,
        l: &'item [T],
        r: &[T],
        mut hash: impl FnMut(&mut EqCtxt, &'item T) -> u64,
    ) -> bool {
        // This has to be the top frame of the stack in order to work.
        debug_assert_eq!(self.cx.unordered.len() - self.start, l.len());

        'outer: for l in l {
            let l_hash = hash(self.cx, l);
            let mut start = self.start;
            while let Some(idx) = self.cx.unordered[start..].iter().position(|&(r, _)| l_hash == r) {
                let r = &r[self.cx.unordered[idx].1];
                if self.cx.eq(l, r) {
                    // The order of elements in a frame doesn't matter so
                    // `swap_remove` is fine.
                    self.cx.unordered.swap_remove(idx);
                    continue 'outer;
                }
                start = idx + 1;
            }
            return false;
        }
        true
    }
}
impl Drop for UnorderedMatcher<'_> {
    fn drop(&mut self) {
        self.cx.unordered.truncate(self.start);
    }
}

pub trait AstNode {
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool;
    fn hash(cx: &mut EqCtxt, item: &Self);
}

impl AstNode for () {
    #[inline]
    fn eq(_: &mut EqCtxt, (): &Self, (): &Self) -> bool {
        true
    }
    #[inline]
    fn hash(_: &mut EqCtxt, (): &Self) {}
}

impl<T: ?Sized + AstNode> AstNode for &T {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        T::eq(cx, l, r)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        T::hash(cx, item);
    }
}
impl<T: ?Sized + AstNode> AstNode for Box<T> {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        T::eq(cx, l, r)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        T::hash(cx, item);
    }
}

impl<T: AstNode, U: AstNode> AstNode for (T, U) {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        T::eq(cx, &l.0, &r.0) && U::eq(cx, &l.1, &r.1)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        T::hash(cx, &item.0);
        U::hash(cx, &item.1);
    }
}
impl<T: AstNode, U: AstNode, V: AstNode> AstNode for (T, U, V) {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        T::eq(cx, &l.0, &r.0) && U::eq(cx, &l.1, &r.1) && V::eq(cx, &l.2, &r.2)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        T::hash(cx, &item.0);
        U::hash(cx, &item.1);
        V::hash(cx, &item.2);
    }
}

impl<T: AstNode> AstNode for [T] {
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        l.len() == r.len() && l.iter().zip(r).all(|(l, r)| T::eq(cx, l, r))
    }
    fn hash(cx: &mut EqCtxt, item: &Self) {
        cx.hasher.write_length_prefix(item.len());
        for x in item {
            T::hash(cx, x);
        }
    }
}
impl<T: AstNode> AstNode for Vec<T> {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        <[T]>::eq(cx, l, r)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        <[T]>::hash(cx, item);
    }
}
impl<T: AstNode> AstNode for ThinVec<T> {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        <[T]>::eq(cx, l, r)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        <[T]>::hash(cx, item);
    }
}

impl<T: ?Sized + ToOwned + AstNode> AstNode for Cow<'_, T> {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        T::eq(cx, l.as_ref(), r.as_ref())
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        T::hash(cx, item.as_ref());
    }
}

/// Implementations for types whose values should be ignored.
macro_rules! ignore_impl {
    ($($ty:ty,)*) => {$(
        impl AstNode for $ty {
            #[inline]
            fn eq(_: &mut EqCtxt, _: &Self, _: &Self) -> bool {
                true
            }
            #[inline]
            fn hash(_: &mut EqCtxt, _: &Self) {}
        }
    )*};
}
ignore_impl!(
    Span,
    NodeId,
    AttrId,
    MatchKind,
    TraitObjectSyntax,
    RangeSyntax,
    DelimSpacing,
);

/// All parse errors are considered unequal.
impl AstNode for ErrorGuaranteed {
    #[inline]
    fn eq(_: &mut EqCtxt, _: &Self, _: &Self) -> bool {
        false
    }
    #[inline]
    fn hash(_: &mut EqCtxt, _: &Self) {}
}

/// Implementations for types which can be forwarded to `PartialEq` and `Hash`.
macro_rules! forward_impl {
    ($($ty:ty,)*) => {$(
        impl AstNode for $ty {
            #[inline]
            fn eq(_: &mut EqCtxt, l: &Self, r: &Self) -> bool {
                *l == *r
            }
            #[inline]
            fn hash(cx: &mut EqCtxt, item: &Self) {
                Hash::hash(item, &mut cx.hasher);
            }
        }
    )*};
}
forward_impl!(
    bool,
    u8,
    u16,
    char,
    usize,
    str,
    Symbol,
    Mutability,
    ByteSymbol,
    Movability,
    InlineAsmOptions,
    FormatTrait,
);

/// Implementations for types which can be forwarded to `PartialEq`, but need
/// to hash their discriminant value.
macro_rules! forward_impl_with_hash_discr {
    ($($ty:ty,)*) => {$(
        impl AstNode for $ty {
            #[inline]
            fn eq(_: &mut EqCtxt, l: &Self, r: &Self) -> bool {
                *l == *r
            }
            #[inline]
            fn hash(cx: &mut EqCtxt, item: &Self) {
                mem::discriminant(item).hash(&mut cx.hasher);
            }
        }
    )*};
}
forward_impl_with_hash_discr!(
    Delimiter,
    AsmMacro,
    LitKind,
    token::LitKind,
    FormatArgPositionKind,
    BinOpKind,
    UnOp,
    FormatAlignment,
    FormatSign,
    FormatDebugHex,
    ForLoopKind,
    GenBlockKind,
    AssignOpKind,
    RangeLimits,
    BorrowKind,
    UnsafeBinderCastKind,
    CommentKind,
    AttrStyle,
    UnsafeSource,
    MacStmtStyle,
    IsAuto,
    PatFieldsRest,
);

macro_rules! eq_field {
    (_ $_field:tt $_cx:ident $_rhs:ident) => {
        true
    };
    ($field:ident $cx:ident $rhs:ident) => {
        AstNode::eq($cx, $field, &$rhs.$field)
    };
    ($field:ident $tuple_idx:tt $cx:ident $rhs:ident) => {
        AstNode::eq($cx, $field, &$rhs.$tuple_idx)
    };
    (@unordered $field:ident $cx:ident $rhs:ident) => {
        $cx.unordered_eq($field, &$rhs.$field)
    };
}
macro_rules! hash_field {
    (_ $_field:ident $_cx:ident $_item:ident) => {};
    ($field:ident $cx:ident $item:ident) => {
        AstNode::hash($cx, &$item.$field)
    };
    ($_field:ident $tuple_idx:tt $cx:ident $item:ident) => {
        AstNode::hash($cx, &$item.$tuple_idx)
    };
    (@unordered $field:ident $cx:ident $item:ident) => {
        $cx.write_unordered_hash(&$item.$field)
    };
}

/// Implementations for structs which will be done by comparing/hashing each field.
///
/// The basic form of the macro calls `eq` and `hash` for each field. e.g.
/// ```ignore
/// StructName {
///     field1,
///     field2,
/// }
/// ```
///
/// Fields can be ignored by using `: _` after the field. e.g.
/// ```ignore
/// StructName {
///     field1,
///     field2,
///     ignored: _,
/// }
/// ```
///
/// Tuple structs have to provide both an index and a name. The names are a syntax
/// limitation and must be unique, but are otherwise meaningless. e.g.
/// ```ignore
/// StructName {
///     0: _0,
///     1: _1,
/// }
/// ```
///
/// Fields which deref into a slice can be compared in an unordered manner by
/// adding `#[unordered]`. e.g.
/// ```ignore
/// StructName {
///     field1,
///     field2,
///     #[unordered] slice_field,
/// }
/// ```
///
/// Generics can be used by adding them after the struct name. Constraints must
/// be parenthesised. e.g.
/// ```ignore
/// StructName<T: (Trait1 + Trait2)> {
///     field1,
///     field2,
/// }
/// ```
macro_rules! impl_struct {
    ($($name:ident $(<$($gargs:tt $(: ($($gbounds:tt)*))?),*>)? {
        $($(#[$unordered:tt])? $fname:tt $(: $capture:tt)?),* $(,)?
    })*) => {$(
        impl $(<$($gargs $(: $($gbounds)*)?),*>)? AstNode for $name $(<$($gargs),*>)? {
            fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
                let $name { $($fname $(: $capture)?),* } = l;
                $(eq_field!($(@$unordered)? $($capture)? $fname cx r))&&*
            }
            fn hash(cx: &mut EqCtxt, item: &Self) {
                $(hash_field!($(@$unordered)? $($capture)? $fname cx item);)*
            }
        }
    )*}
}

macro_rules! eq_enum_field {
    ($cx:ident _ _) => {
        true
    };
    ($cx:ident $lname:ident $rname:ident) => {
        AstNode::eq($cx, $lname, $rname)
    };
    (@unordered $cx:ident $lname:ident $rname:ident) => {
        $cx.unordered_eq($lname, $rname)
    };
    (@unordered_pat_fields $cx:ident $lname:ident $rname:ident) => {
        $cx.eq_pat_fields($lname, $rname)
    };
}
macro_rules! hash_enum_field {
    ($cx:ident _) => {};
    ($cx:ident $name:ident) => {
        AstNode::hash($cx, $name)
    };
    (@unordered $cx:ident $name:ident) => {
        $cx.write_unordered_hash($name)
    };
    (@unordered_pat_fields $cx:ident $name:ident) => {
        $cx.write_unordered_hash($name)
    };
}
macro_rules! on_unequal {
    (@unequal $($val:tt)*) => { $($val)? };
}

/// Implementations for enums which will be done by matching/hashing the variant and
/// each of it's fields.
///
/// The basic form of the macro matches/hashes the variant, and calls `eq` and `hash`
/// for each of it's fields. Each field must be given a pair of unique names. e.g.
/// ```ignore
/// SomeType {
///     UnitVariant {},
///     StructVariant {
///         field1: (l0, r0),
///         field2: (l1, r1),
///     },
///     TupleVariant {
///         0: (l0, r0),
///         1: (l1, r1),
///     },
/// }
/// ```
///
/// A variant can always be compared as unequal by adding the `#[unequal]`. e.g.
/// ```ignore
/// SomeType {
///     UnitVariant {},
///     #[unequal] Dummy {},
/// }
/// ```
///
/// Fields can be ignored by using `_` for both names after the field. e.g.
/// ```ignore
/// SomeType {
///     StructVariant {
///         field1: (l0, r0),
///         ignored: (_, _),
///     },
/// }
/// ```
///
/// Fields which deref into a slice can be compared in an unordered manner by
/// adding `#[unordered]`. e.g.
/// ```ignore
/// SomeType {
///     Var {
///         #[unordered] field: (l0, r0),
///     },
/// }
/// ```
///
/// Generics can be used by adding them after the struct name. Constraints must
/// be parenthesised. e.g.
/// ```ignore
/// StructName<T: (Trait1 + Trait2)> {
///     Var {
///         field: (l0, r0),
///     },
/// }
/// ```
macro_rules! impl_enum {
    ($($name:ident $(<$($gargs:ident $(: ($($gbounds:tt)*))?),*>)? {$(
        $(#[$unequal:tt])?
        $vname:ident {
            $($(#[$unordered:tt])? .$fname:tt: ($lname:tt, $rname:tt),)*
        },
    )*})*) => {$(
        #[allow(non_snake_case, unreachable_code)]
        impl  $(<$($gargs $(: $($gbounds)*)?),*>)? AstNode for $name $(<$($gargs),*>)? {
            fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
                match (l, r) {
                    $(
                        (
                            $name::$vname { $($fname: $lname),*},
                            $name::$vname { $($fname: $rname),*}
                        ) => {
                            $(on_unequal!(@$unequal return false);)?
                            $(eq_enum_field!($(@$unordered)? cx $lname $rname) &&)* true
                        },
                    )*
                    _ => false,
                }
            }
            fn hash(cx: &mut EqCtxt, $name: &Self) {
                mem::discriminant($name).hash(&mut cx.hasher);
                match $name {$(
                    $name::$vname {$($fname: $lname),*} => {
                        $(on_unequal!(@$unequal return);)?
                        $(hash_enum_field!($(@$unordered)? cx $lname);)*}
                ),*}
            }
        }
    )*};
}

impl_struct! {
    Ident {
        name,
        span,
    }
    Spanned<T: (AstNode)> {
        node,
        span,
    }
    Label {
        ident,
    }
    PatField {
        ident,
        is_placeholder,
        pat,
        attrs,
        id,
        span,
        is_shorthand: _,
    }
    QSelf {
        ty,
        position,
        path_span,
    }
    Path {
        segments,
        span,
        tokens: _,
    }
    PathSegment {
        ident,
        args,
        id,
    }
    AngleBracketedArgs {
        args,
        span,
    }
    ParenthesizedArgs {
        inputs,
        output,
        span,
        inputs_span,
    }
    StructExpr {
        qself,
        path,
        fields,
        rest,
    }
    MethodCall {
        seg,
        receiver,
        args,
        span,
    }
    Closure {
        binder,
        capture_clause,
        constness,
        coroutine_kind,
        movability,
        fn_decl,
        body,
        fn_decl_span,
        fn_arg_span,
    }
    ExprField {
        ident,
        is_placeholder,
        expr,
        attrs,
        id,
        span,
        is_shorthand: _,
    }
    Arm {
        is_placeholder,
        pat,
        body,
        guard,
        attrs,
        id,
        span,
    }
    Block {
        rules,
        stmts,
        id,
        span,
        tokens: _,
    }
    MacCallStmt {
        mac,
        style,
        attrs,
        tokens: _,
    }
    Trait {
        constness,
        safety,
        is_auto,
        ident,
        generics,
        bounds,
        items,
    }
    Impl {
        generics,
        of_trait,
        self_ty,
        items,
    }
    ForeignMod {
        safety,
        abi,
        items,
        extern_span,
    }
    TraitRef {
        path,
        ref_id,
    }
    TraitImplHeader {
        defaultness,
        safety,
        constness,
        polarity,
        trait_ref,
    }
    StaticItem {
        ident,
        mutability,
        safety,
        ty,
        expr,
        define_opaque: _,
    }
    Fn {
        ident,
        defaultness,
        sig,
        generics,
        contract,
        body,
        define_opaque: _,
    }
    TyAlias {
        ident,
        defaultness,
        generics,
        bounds,
        ty,
        where_clauses: _,
    }
    ConstItem {
        ident,
        defaultness,
        generics,
        ty,
        expr,
        define_opaque: _,
    }
    Variant {
        ident,
        vis,
        is_placeholder,
        data,
        disr_expr,
        attrs,
        id,
        span,
    }
    FieldDef {
        ident,
        vis,
        safety,
        is_placeholder,
        ty,
        default,
        attrs,
        id,
        span,
    }
    FnSig {
        header,
        decl,
        span,
    }
    FnHeader {
        safety,
        coroutine_kind,
        constness,
        ext,
    }
    FnContract {
        requires,
        ensures,
    }
    Generics {
        params,
        where_clause,
        span,
    }
    WhereClause {
        predicates,
        span,
        has_where_token: _,
    }
    WherePredicate {
        kind,
        is_placeholder,
        attrs,
        id,
        span,
    }
    WhereBoundPredicate {
        bounded_ty,
        bound_generic_params,
        bounds,
    }
    WhereRegionPredicate {
        lifetime,
        bounds,
    }
    WhereEqPredicate {
        lhs_ty,
        rhs_ty,
    }
    Lifetime {
        ident,
        id,
    }
    UseTree {
        prefix,
        kind,
        span,
    }
    AnonConst {
        value,
        id,
    }
    FnDecl {
        inputs,
        output,
    }
    Param {
        pat,
        is_placeholder,
        ty,
        attrs,
        id,
        span,
    }
    MutTy {
        ty,
        mutbl,
    }
    FnPtrTy {
        safety,
        ext,
        generic_params,
        decl,
        decl_span,
    }
    StrLit {
        style,
        symbol,
        suffix,
        span,
        symbol_unescaped: _,
    }
    PolyTraitRef {
        modifiers,
        trait_ref,
        bound_generic_params,
        span,
        parens: _,
    }
    TraitBoundModifiers {
        constness,
        asyncness,
        polarity,
    }
    GenericParam {
        ident,
        is_placeholder,
        bounds,
        kind,
        attrs,
        id,
        colon_span,
    }
    AssocItemConstraint {
        ident,
        gen_args,
        kind,
        id,
        span,
    }
    MacCall {
        path,
        args,
    }
    Attribute {
        style,
        kind,
        id,
        span,
    }
    NormalAttr {
        item,
        tokens: _,
    }
    AttrItem {
        path,
        unsafety,
        args,
        tokens: _,
    }
    DelimArgs {
        delim,
        tokens,
        dspan,
    }
    Lit {
        kind,
        symbol,
        suffix,
    }
    Delegation {
        ident,
        qself,
        path,
        rename,
        body,
        from_glob,
        id,
    }
    DelegationMac {
        qself,
        prefix,
        suffixes,
        body,
    }
    TyPat {
        kind,
        id,
        span,
        tokens: _,
    }
    Token {
        kind,
        span,
    }
    Visibility {
        kind,
        span,
        tokens: _,
    }
    InlineAsm {
        asm_macro,
        template,
        template_strs,
        operands,
        clobber_abis,
        options,
        line_spans: _,
    }
    ModSpans {
        inner_span,
        inject_use_span,
    }
    EnumDef {
        variants,
    }
    DelimSpan {
        open,
        close,
    }
    MacroDef {
        body,
        macro_rules,
    }
    UnsafeBinderTy {
        generic_params,
        inner_ty,
    }
    Local {
        id,
        pat,
        ty,
        kind,
        span,
        attrs,
        colon_sp: _,
        super_: _,
        tokens: _,
    }
    FormatArgs {
        span,
        template,
        arguments,
        is_source_literal,
        uncooked_fmt_str: _,
    }
    FormatPlaceholder {
        argument,
        span,
        format_trait,
        format_options,
    }
    FormatArgPosition {
        index,
        kind,
        span: _,
    }
    FormatArgument {
        kind,
        expr,
    }
    FormatOptions {
        width,
        precision,
        alignment,
        fill,
        sign,
        alternate,
        zero_pad,
        debug_hex,
    }
    InlineAsmSym {
        id,
        qself,
        path,
    }
    Item<K: (AstNode)> {
        id,
        span,
        vis,
        kind,
        attrs,
        tokens: _,
    }
    BindingMode {
        0: f0,
        1: f1,
    }
}

impl_enum! {
    Option<T: (AstNode)> {
        Some { .0: (l0, r0), },
        None {},
    }
    Result<T: (AstNode), U: (AstNode)> {
        Ok { .0: (l0, r0), },
        Err { .0: (l0, r0), },
    }
    Term {
        Ty { .0: (l0, r0), },
        Const { .0: (l0, r0), },
    }
    GenericArgs {
        AngleBracketed { .0: (l0, r0), },
        Parenthesized { .0: (l0, r0), },
        ParenthesizedElided { .0: (l0, r0), },
    }
    AngleBracketedArg {
        Arg { .0: (l0, r0), },
        Constraint { .0: (l0, r0), },
    }
    GenericArg {
        Lifetime { .0: (l0, r0), },
        Type { .0: (l0, r0), },
        Const { .0: (l0, r0), },
    }
    StructRest {
        Base { .0: (l0, r0), },
        Rest { .0: (l0, r0), },
        None {},
    }
    CaptureBy {
        Ref {},
        Use { .use_kw: (l0, r0), },
        Value { .move_kw: (l0, r0), },
    }
    CoroutineKind {
        Async {
            .span: (l0, r0),
            .closure_id: (l1, r1),
            .return_impl_trait_id: (l2, r2),
        },
        Gen {
            .span: (l0, r0),
            .closure_id: (l1, r1),
            .return_impl_trait_id: (l2, r2),
        },
        AsyncGen {
            .span: (l0, r0),
            .closure_id: (l1, r1),
            .return_impl_trait_id: (l2, r2),
        },
    }
    LocalKind {
        Decl {},
        Init { .0: (l0, r0), },
        InitElse {
            .0: (l0, r0),
            .1: (l1, r1),
        },
    }
    ModKind {
        Loaded {
            .1: (l1, r1),
            .0: (l0, r0),
            .2: (l2, r2),
        },
        Unloaded {},
    }
    ItemKind {
        ExternCrate {
            .1: (l1, r1),
            .0: (l0, r0),
        },
        Use { .0: (l0, r0), },
        Static { .0: (l0, r0), },
        Const { .0: (l0, r0), },
        Fn { .0: (l0, r0), },
        Mod {
            .1: (l1, r1),
            .0: (l0, r0),
            .2: (l2, r2),
        },
        ForeignMod { .0: (l0, r0), },
        TyAlias { .0: (l0, r0), },
        Enum {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Struct {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Union {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Trait { .0: (l0, r0), },
        TraitAlias {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Impl { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
        MacroDef {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Delegation { .0: (l0, r0), },
        DelegationMac { .0: (l0, r0), },
        GlobalAsm { .0: (l0, r0), },
    }
    Safety {
        Default {},
        Safe { .0: (l0, r0), },
        Unsafe { .0: (l0, r0), },
    }
    Const {
        Yes { .0: (l0, r0), },
        No {},
    }
    ImplPolarity {
        Positive {},
        Negative { .0: (l0, r0), },
    }
    ForeignItemKind {
        Static { .0: (l0, r0), },
        Fn { .0: (l0, r0), },
        TyAlias { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
    }
    AssocItemKind {
        Const { .0: (l0, r0), },
        Fn { .0: (l0, r0), },
        Type { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
        Delegation { .0: (l0, r0), },
        DelegationMac { .0: (l0, r0), },
    }
    VariantData {
        Unit { .0: (l0, r0), },
        Struct {
            .fields: (l0, r0),
            .recovered: (l1, r1),
        },
        Tuple {
            .0: (l0, r0),
            .1: (l1, r1),
        },
    }
    WherePredicateKind {
        BoundPredicate { .0: (l0, r0), },
        RegionPredicate { .0: (l0, r0), },
        EqPredicate { .0: (l0, r0), },
    }
    UseTreeKind {
        Glob {},
        Simple { .0: (l0, r0), },
        Nested {
            .items: (l0, r0),
            .span: (l1, r1),
        },
    }
    Defaultness {
        Final {},
        Default { .0: (l0, r0), },
    }
    VisibilityKind {
        Public {},
        Inherited {},
        Restricted {
            .path: (l0, r0),
            .id: (l1, r1),
            .shorthand: (l2, r2),
        },
    }
    ClosureBinder {
        NotPresent {},
        For {
            .generic_params: (l0, r0),
            .span: (l1, r1),
        },
    }
    FnRetTy {
        Default { .0: (l0, r0), },
        Ty { .0: (l0, r0), },
    }
    TyKind {
        Never {},
        ImplicitSelf {},
        CVarArgs {},
        Paren { .0: (l0, r0), },
        Slice { .0: (l0, r0), },
        Ptr { .0: (l0, r0), },
        FnPtr { .0: (l0, r0), },
        Tup { .0: (l0, r0), },
        Typeof { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
        UnsafeBinder { .0: (l0, r0), },
        Err { .0: (l0, r0), },
        Array {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Ref {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        PinnedRef {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Path {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Pat {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        TraitObject {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        ImplTrait {
            .1: (l1, r1),
            .0: (l0, r0),
        },
        #[unequal] Infer {},
        #[unequal] Dummy {},
    }
    TyPatKind {
        Range {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Or { #[unordered] .0: (l0, r0), },
        Err { .0: (l0, r0), },
    }
    Extern {
        None {},
        Implicit { .0: (l0, r0), },
        Explicit {
            .0: (l0, r0),
            .1: (l1, r1),
        },
    }
    BoundConstness {
        Never {},
        Always { .0: (l0, r0), },
        Maybe { .0: (l0, r0), },
    }
    BoundAsyncness {
        Normal {},
        Async { .0: (l0, r0), },
    }
    BoundPolarity {
        Positive {},
        Negative { .0: (l0, r0), },
        Maybe { .0: (l0, r0), },
    }
    GenericParamKind {
        Lifetime {},
        Type { .default: (l0, r0), },
        Const {
            .ty: (l0, r0),
            .default: (l1, r1),
            .span: (l2, r2),
        },
    }
    GenericBound {
        Trait { .0: (l0, r0), },
        Outlives { .0: (l0, r0), },
        Use {
            .0: (l0, r0),
            .1: (l1, r1),
        },
    }
    PreciseCapturingArg {
        Lifetime { .0: (l0, r0), },
        Arg {
            .0: (l0, r0),
            .1: (l1, r1),
        },
    }
    AssocItemConstraintKind {
        Equality { .term: (l0, r0), },
        Bound { .bounds: (l0, r0), },
    }
    AttrKind {
        DocComment {
            .1: (l1, r1),
            .0: (l0, r0),
        },
        Normal { .0: (l0, r0), },
    }
    AttrArgs {
        Empty {},
        Delimited { .0: (l0, r0), },
        Eq {
            .expr: (l0, r0),
            .eq_span: (l1, r1),
        },
    }
    TokenTree {
        Token {
            .1: (l1, r1),
            .0: (l0, r0),
        },
        Delimited {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
            .3: (l3, r3),
        },
    }
    StmtKind {
        Empty {},
        Let { .0: (l0, r0), },
        Item { .0: (l0, r0), },
        Expr { .0: (l0, r0), },
        Semi { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
    }
    ExprKind {
        Underscore {},
        Err { .0: (l0, r0), },
        Array { .0: (l0, r0), },
        ConstBlock { .0: (l0, r0), },
        MethodCall { .0: (l0, r0), },
        Tup { .0: (l0, r0), },
        Lit { .0: (l0, r0), },
        Closure { .0: (l0, r0), },
        TryBlock { .0: (l0, r0), },
        Continue { .0: (l0, r0), },
        Ret { .0: (l0, r0), },
        InlineAsm { .0: (l0, r0), },
        MacCall { .0: (l0, r0), },
        Struct { .0: (l0, r0), },
        Paren { .0: (l0, r0), },
        Try { .0: (l0, r0), },
        Yield { .0: (l0, r0), },
        Yeet { .0: (l0, r0), },
        Become { .0: (l0, r0), },
        IncludedBytes { .0: (l0, r0), },
        FormatArgs { .0: (l0, r0), },
        Call {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Binary {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Unary {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Cast {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Type {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Let {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
            .3: (l3, r3),
        },
        If {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        While {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        ForLoop {
            .pat: (l0, r0),
            .iter: (l1, r1),
            .label: (l2, r2),
            .kind: (l3, r3),
            .body: (l4, r4),
        },
        Loop {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Match {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Block {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Gen {
            .0: (l0, r0),
            .2: (l2, r2),
            .1: (l1, r1),
            .3: (l3, r3),
        },
        Await {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Use {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Assign {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        AssignOp {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Field {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Index {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Range {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Path {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        AddrOf {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Break {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        OffsetOf {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Repeat {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        UnsafeBinderCast {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        #[unequal] Dummy {},
    }
    InlineAsmTemplatePiece {
        String { .0: (l0, r0), },
        Placeholder {
            .operand_idx: (l0, r0),
            .modifier: (l1, r1),
            .span: (l2, r2),
        },
    }
    InlineAsmOperand {
        In {
            .reg: (l0, r0),
            .expr: (l1, r1),
        },
        Out {
            .reg: (l0, r0),
            .late: (l1, r1),
            .expr: (l2, r2),
        },
        InOut {
            .reg: (l0, r0),
            .late: (l1, r1),
            .expr: (l2, r2),
        },
        SplitInOut {
            .reg: (l0, r0),
            .late: (l1, r1),
            .in_expr: (l2, r2),
            .out_expr: (l3, r3),
        },
        Const { .anon_const: (l0, r0), },
        Sym { .sym: (l0, r0), },
        Label { .block: (l0, r0), },
    }
    InlineAsmRegOrRegClass {
        Reg { .0: (l0, r0), },
        RegClass { .0: (l0, r0), },
    }
    Inline {
        Yes {},
        No { .had_parse_error: (l0, r0), },
    }
    BlockCheckMode {
        Default {},
        Unsafe { .0: (l0, r0), },
    }
    StrStyle {
        Cooked {},
        Raw { .0: (l0, r0), },
    }
    Recovered {
        No {},
        Yes { .0: (l0, r0), },
    }
    RangeEnd {
        Included { .0: (l0, r0), },
        Excluded {},
    }
    FormatArgsPiece {
        Literal { .0: (l0, r0), },
        Placeholder { .0: (l0, r0), },
    }
    FormatArgumentKind {
        Normal {},
        Named { .0: (l0, r0), },
        Captured { .0: (l0, r0), },
    }
    FormatCount {
        Literal { .0: (l0, r0), },
        Argument { .0: (l0, r0), },
    }
    PatKind {
        Paren { .0: (l0, r0), },
        Err { .0: (l0, r0), },
        Wild {},
        Rest {},
        Never {},
        Expr { .0: (l0, r0), },
        Ident {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Range {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Box { .0: (l0, r0), },
        Deref { .0: (l0, r0), },
        Ref {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        Tuple { .0: (l0, r0), },
        Slice { .0: (l0, r0), },
        Path {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        TupleStruct {
            .0: (l0, r0),
            .1: (l1, r1),
            .2: (l2, r2),
        },
        Struct {
            .0: (l0, r0),
            .1: (l1, r1),
            .3: (l3, r3),
            #[unordered_pat_fields] .2: (l2, r2),
        },
        Or {
            #[unordered] .0: (l0, r0),
        },
        MacCall { .0: (l0, r0), },
        Guard {
            .0: (l0, r0),
            .1: (l1, r1),
        },
        #[unequal] Missing {},
    }
    ByRef {
        Yes { .0: (l0, r0), },
        No {},
    }
}

impl AstNode for Pat {
    fn eq(cx: &mut EqCtxt, mut l: &Self, mut r: &Self) -> bool {
        while let PatKind::Paren(x) = &l.kind {
            l = &**x;
        }
        while let PatKind::Paren(x) = &r.kind {
            r = &**x;
        }
        PatKind::eq(cx, &l.kind, &r.kind)
    }

    fn hash(cx: &mut EqCtxt, mut item: &Self) {
        while let PatKind::Paren(x) = &item.kind {
            item = &**x;
        }
        PatKind::hash(cx, &item.kind);
    }
}

impl AstNode for Expr {
    fn eq(cx: &mut EqCtxt, mut l: &Self, mut r: &Self) -> bool {
        while l.attrs.is_empty()
            && let ExprKind::Paren(x) = &l.kind
        {
            l = &**x;
        }
        while r.attrs.is_empty()
            && let ExprKind::Paren(x) = &r.kind
        {
            r = &**x;
        }
        ExprKind::eq(cx, &l.kind, &r.kind) && AstNode::eq(cx, &l.attrs, &r.attrs)
    }

    fn hash(cx: &mut EqCtxt, mut item: &Self) {
        while item.attrs.is_empty()
            && let ExprKind::Paren(x) = &item.kind
        {
            item = &**x;
        }
        ExprKind::hash(cx, &item.kind);
    }
}

impl AstNode for YieldKind {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        AstNode::eq(cx, &l.expr(), &r.expr())
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        AstNode::hash(cx, &item.expr());
    }
}

impl AstNode for Stmt {
    #[inline]
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        StmtKind::eq(cx, &l.kind, &r.kind)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        StmtKind::hash(cx, &item.kind);
    }
}

impl AstNode for Ty {
    fn eq(cx: &mut EqCtxt, mut l: &Self, mut r: &Self) -> bool {
        while let TyKind::Paren(x) = &l.kind {
            l = &**x;
        }
        while let TyKind::Paren(x) = &r.kind {
            r = &**x;
        }
        TyKind::eq(cx, &l.kind, &r.kind)
    }

    fn hash(cx: &mut EqCtxt, mut item: &Self) {
        while let TyKind::Paren(x) = &item.kind {
            item = &**x;
        }
        TyKind::hash(cx, &item.kind);
    }
}

impl AstNode for TokenStream {
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        l.len() == r.len() && l.iter().zip(r.iter()).all(|(l, r)| cx.eq(l, r))
    }

    fn hash(cx: &mut EqCtxt, item: &Self) {
        cx.hasher.write_length_prefix(item.len());
        for item in item.iter() {
            AstNode::hash(cx, item);
        }
    }
}

impl AstNode for TokenKind {
    #[inline]
    fn eq(_: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        *l == *r
    }

    fn hash(cx: &mut EqCtxt, item: &Self) {
        use rustc_ast::token::TokenKind::*;
        mem::discriminant(item).hash(&mut cx.hasher);
        match item {
            Ident(x, _) | Lifetime(x, _) | DocComment(_, _, x) => x.hash(&mut cx.hasher),
            NtIdent(x, _) | NtLifetime(x, _) => AstNode::hash(cx, x),
            Literal(x) => AstNode::hash(cx, x),
            _ => {},
        }
    }
}

impl AstNode for Spacing {
    #[inline]
    fn eq(_: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        matches!(*l, Spacing::Joint) == matches!(*r, Spacing::Joint)
    }
    #[inline]
    fn hash(cx: &mut EqCtxt, item: &Self) {
        matches!(*item, Spacing::Joint).hash(&mut cx.hasher);
    }
}

impl AstNode for FormatArguments {
    fn eq(cx: &mut EqCtxt, l: &Self, r: &Self) -> bool {
        l.all_args().len() == r.all_args().len()
            && AstNode::eq(cx, l.unnamed_args(), r.unnamed_args())
            && cx.unordered_eq_by_key(l.named_args(), r.named_args(), |x| &x.kind)
            && cx.unordered_eq_by_key(
                &l.all_args()[l.explicit_args().len()..],
                &r.all_args()[r.explicit_args().len()..],
                |x| &x.kind,
            )
    }

    fn hash(cx: &mut EqCtxt, item: &Self) {
        AstNode::hash(cx, item.unnamed_args());
        cx.write_unordered_hash(item.named_args());
        cx.write_unordered_hash(&item.all_args()[item.explicit_args().len()..]);
    }
}
