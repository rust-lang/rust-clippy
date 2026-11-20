use crate::projection::{self, PlaceData, ResolvedPlace as _, Resolver};
use core::ops::Range;
use rustc_abi::{FieldIdx, VariantIdx};
use rustc_index::{Idx, IndexSlice};
use rustc_middle::mir::{
    AggregateKind, BasicBlockData, BinOp, Body, BorrowKind, CastKind, ConstOperand, CopyNonOverlapping,
    InlineAsmOperand, Local, NonDivergingIntrinsic, NullOp, Operand, Place, RETURN_PLACE, RawPtrKind, Rvalue,
    Statement, StatementKind, Terminator, TerminatorKind, UnOp,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::source_map::Spanned;
use rustc_span::{Span, sym};

/// Visitor for tracking the movement of values within a MIR body.
pub trait Visitor<'arena, 'tcx>: Sized {
    type Resolver: Resolver<'arena>;
    fn resolver(&self) -> &Self::Resolver;

    /// Gets the `TyCtxt` this visitor instance is associated with.
    fn tcx(&self) -> TyCtxt<'tcx>;

    /// Gets the MIR body this visitor instance is associated with.
    fn body(&self) -> &Body<'tcx>;

    /// Visits a read of an individual value.
    fn visit_read_idx(&mut self, idx: projection::Idx, sp: Span);

    /// Visits a mutation of an individual value.
    fn visit_mutate_idx(&mut self, idx: projection::Idx, sp: Span);

    /// Visits a write of `uninit` bytes to an individual value.
    fn visit_uninit_idx(&mut self, idx: projection::Idx, sp: Span);

    /// Visits a copy of one value to another.
    fn visit_copy_idx(&mut self, dst: projection::Idx, src: projection::Idx, sp: Span);

    /// Visits a move of one value to another.
    fn visit_move_idx(&mut self, dst: projection::Idx, src: projection::Idx, sp: Span);

    /// Visits a move of an individual value to an unknown place.
    ///
    /// Default to calling `visit_read_idx` followed by `visit_uninit_idx`.
    #[inline]
    fn visit_consume_idx(&mut self, idx: projection::Idx, sp: Span) {
        self.visit_read_idx(idx, sp);
        self.visit_uninit_idx(idx, sp);
    }

    /// Visits a read of a set of values.
    #[inline]
    fn visit_read_range(&mut self, range: Range<projection::Idx>, sp: Span) {
        for i in range {
            self.visit_read_idx(i, sp);
        }
    }

    /// Visits a mutation of a set of values.
    #[inline]
    fn visit_mutate_range(&mut self, range: Range<projection::Idx>, sp: Span) {
        for i in range {
            self.visit_mutate_idx(i, sp);
        }
    }

    /// Visits a write of `uninit` bytes to a set of values.
    #[inline]
    fn visit_uninit_range(&mut self, range: Range<projection::Idx>, sp: Span) {
        for i in range {
            self.visit_uninit_idx(i, sp);
        }
    }

    /// Visits a copy from one set of values to another.
    #[inline]
    fn visit_copy_range(&mut self, dst: projection::Idx, src: Range<projection::Idx>, sp: Span) {
        for (dst, src) in (dst..).zip(src) {
            self.visit_copy_idx(dst, src, sp);
        }
    }

    /// Visits a move from one set of values to another.
    #[inline]
    fn visit_move_range(&mut self, dst: projection::Idx, src: Range<projection::Idx>, sp: Span) {
        for (dst, src) in (dst..).zip(src) {
            self.visit_move_idx(dst, src, sp);
        }
    }

    /// Visits a move of a set of values to an unknown place.
    #[inline]
    fn visit_consume_range(&mut self, range: Range<projection::Idx>, sp: Span) {
        for i in range {
            self.visit_consume_idx(i, sp);
        }
    }

    /// Visits the parent of a read field.
    #[inline]
    fn visit_read_parent(&mut self, idx: projection::Idx, sp: Span) {
        self.visit_read_idx(idx, sp);
    }

    /// Visits the parent of a mutated field.
    #[inline]
    fn visit_mutate_parent(&mut self, idx: projection::Idx, sp: Span) {
        self.visit_mutate_idx(idx, sp);
    }

    /// Visits the parent of a consumed field.
    ///
    /// Defaults to calling `visit_read_parent` followed by `visit_mutate_parent`.
    #[inline]
    fn visit_consume_parent(&mut self, idx: projection::Idx, sp: Span) {
        self.visit_read_parent(idx, sp);
        self.visit_mutate_parent(idx, sp);
    }

    /// Visits a read of a resolved place.
    ///
    /// Defaults to calling `visit_read_range` for the contained values and `visit_read_idx` for
    /// each parent.
    fn visit_read_place(&mut self, place: Place<'tcx>, sp: Span) {
        let place = self.resolver().resolve(place);
        let (start, data) = place.values();
        if data.contains_values() {
            self.visit_read_range(start..start.plus(data.value_count as usize), sp);
        }
        for idx in place.parents(self.resolver()) {
            self.visit_read_parent(idx, sp);
        }
    }

    /// Visits a mutation of a resolved place.
    ///
    /// Defaults to calling `visit_mutate_range` for the contained values and `visit_mutate_idx` for
    /// each parent.
    fn visit_mutate_place(&mut self, place: Place<'tcx>, sp: Span) {
        let place = self.resolver().resolve(place);
        let (start, data) = place.values();
        if data.contains_values() {
            self.visit_mutate_range(start..start.plus(data.value_count as usize), sp);
        }
        for idx in place.parents(self.resolver()) {
            self.visit_mutate_parent(idx, sp);
        }
    }

    /// Visits a write of `uninit` bytes to a resolved place.
    ///
    /// Defaults to calling `visit_uninit_range` for the contained values and `visit_mutate_idx` for
    /// each parent.
    fn visit_uninit_place(&mut self, place: Place<'tcx>, sp: Span) {
        let place = self.resolver().resolve(place);
        let (start, data) = place.values();
        if data.contains_values() {
            self.visit_uninit_range(start..start.plus(data.value_count as usize), sp);
        }
        for idx in place.parents(self.resolver()) {
            self.visit_mutate_parent(idx, sp);
        }
    }

    /// Visits a copy from one resolved place to another.
    fn visit_copy_place(&mut self, dst: Place<'tcx>, src: Place<'tcx>, sp: Span) {
        let dst = self.resolver().resolve(dst);
        let src = self.resolver().resolve(src);
        let (dst_start, dst_data) = dst.values();
        let (src_start, src_data) = src.values();
        if dst_data.contains_values() || src_data.contains_values() {
            copy_place(self, Copy, dst_start, dst_data, src_start, src_data, sp);
        }
        for idx in src.parents(self.resolver()) {
            self.visit_read_parent(idx, sp);
        }
        for idx in dst.parents(self.resolver()) {
            self.visit_mutate_parent(idx, sp);
        }
    }

    /// Visits a move from one resolved place to another.
    fn visit_move_place(&mut self, dst: Place<'tcx>, src: Place<'tcx>, sp: Span) {
        let dst = self.resolver().resolve(dst);
        let src = self.resolver().resolve(src);
        let (dst_start, dst_data) = dst.values();
        let (src_start, src_data) = src.values();
        if dst_data.contains_values() || src_data.contains_values() {
            copy_place(self, Move, dst_start, dst_data, src_start, src_data, sp);
        }
        for idx in src.parents(self.resolver()) {
            self.visit_consume_parent(idx, sp);
        }
        for idx in dst.parents(self.resolver()) {
            self.visit_mutate_parent(idx, sp);
        }
    }

    /// Visits a move from a resolved place to an unknown location.
    ///
    /// Defaults to calling `visit_consume_range` followed by `visit_read_idx` and
    /// `visit_mutate_idx` for each parent.
    fn visit_consume_place(&mut self, place: Place<'tcx>, sp: Span) {
        let place = self.resolver().resolve(place);
        let (start, data) = place.values();
        if data.contains_values() {
            self.visit_consume_range(start..start.plus(data.value_count as usize), sp);
        }
        for idx in place.parents(self.resolver()) {
            self.visit_consume_parent(idx, sp);
        }
    }

    /// Visits a drop of a resolved place.
    ///
    /// Defaults to calling `visit_uninit_place`.
    #[inline]
    fn visit_drop_place(&mut self, place: Place<'tcx>, sp: Span) {
        self.visit_uninit_place(place, sp);
    }

    #[inline]
    fn visit_uninit_local(&mut self, local: Local, sp: Span) {
        let (start, data) = self.resolver().resolve_local(local);
        if data.contains_values() {
            self.visit_uninit_range(start..start.plus(data.value_count as usize), sp);
        }
    }

    #[inline]
    fn visit_consume_local(&mut self, local: Local, sp: Span) {
        let (start, data) = self.resolver().resolve_local(local);
        if data.contains_values() {
            self.visit_consume_range(start..start.plus(data.value_count as usize), sp);
        }
    }

    #[inline]
    fn visit_assign_constant_field(
        &mut self,
        dst_start: projection::Idx,
        dst_data: &PlaceData<'_>,
        _src: &ConstOperand<'tcx>,
        sp: Span,
    ) {
        if dst_data.contains_values() {
            self.visit_mutate_range(dst_start..dst_start.plus(dst_data.value_count as usize), sp);
        }
    }

    fn visit_assign_aggregate(
        &mut self,
        dst: Place<'tcx>,
        _kind: &AggregateKind<'tcx>,
        ops: &IndexSlice<FieldIdx, Operand<'tcx>>,
        sp: Span,
    ) {
        let dst = self.resolver().resolve(dst);
        let (dst_start, dst_data) = dst.values();
        if dst_data.contains_values() && dst_data.value_count > u32::from(dst_data.has_value) {
            debug_assert_eq!(dst_data.fields.len(), ops.len());
            for (dst_field, op) in dst_data.fields.iter().zip(ops) {
                if dst_field.data.contains_values() {
                    let dst = dst_start.plus(dst_field.offset as usize);
                    match op {
                        &Operand::Copy(src) => {
                            copy_aggregate_field(self, Copy, dst, dst_field.data, self.resolver().resolve(src), sp);
                        },
                        &Operand::Move(src) => {
                            copy_aggregate_field(self, Move, dst, dst_field.data, self.resolver().resolve(src), sp);
                        },
                        Operand::Constant(src) => self.visit_assign_constant_field(dst, dst_field.data, src, sp),
                    }
                }
            }
        } else {
            for op in ops {
                walk_operand(self, op, sp);
            }
        }
        for idx in dst.parents(self.resolver()) {
            self.visit_mutate_idx(idx, sp);
        }
    }

    #[inline]
    fn visit_assign_unary_op(&mut self, dst: Place<'tcx>, _op: UnOp, src: &Operand<'tcx>, sp: Span) {
        walk_operand(self, src, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_binary_op(
        &mut self,
        dst: Place<'tcx>,
        _op: BinOp,
        (lhs, rhs): &(Operand<'tcx>, Operand<'tcx>),
        sp: Span,
    ) {
        walk_operand(self, lhs, sp);
        walk_operand(self, rhs, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_cast(&mut self, dst: Place<'tcx>, _kind: CastKind, src: &Operand<'tcx>, _ty: Ty<'tcx>, sp: Span) {
        walk_operand(self, src, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_len(&mut self, dst: Place<'tcx>, src: Place<'tcx>, sp: Span) {
        self.visit_read_place(src, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_discriminant(&mut self, dst: Place<'tcx>, src: Place<'tcx>, sp: Span) {
        self.visit_read_place(src, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_null_op(&mut self, dst: Place<'tcx>, _op: &NullOp<'tcx>, _ty: Ty<'tcx>, sp: Span) {
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_raw_ptr(&mut self, dst: Place<'tcx>, src: Place<'tcx>, _kind: RawPtrKind, sp: Span) {
        // A raw borrow can invalidate any value tracking done unless special care is taken.
        debug_assert!(
            !self.resolver().resolve(src).affects_any_value(),
            "A raw borrow of a tracked place was taken at `{sp:?}`. \
                Use `clippy_mir::projection::create_raw_borrow_filter` to filter out these places.",
        );
        self.visit_mutate_place(dst, sp);
    }

    fn visit_assign_borrow(&mut self, dst: Place<'tcx>, src: Place<'tcx>, kind: BorrowKind, sp: Span) {
        let src = self.resolver().resolve(src);
        let (src_start, src_data) = src.values();
        if src_data.contains_values() {
            let src_range = src_start..src_start.plus(src_data.value_count as usize);
            if matches!(kind, BorrowKind::Mut { .. }) {
                self.visit_mutate_range(src_range.clone(), sp);
            }
            self.visit_read_range(src_range, sp);
        }
        if matches!(kind, BorrowKind::Mut { .. }) {
            for idx in src.parents(self.resolver()) {
                self.visit_mutate_parent(idx, sp);
            }
        }
        for idx in src.parents(self.resolver()) {
            self.visit_read_parent(idx, sp);
        }
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_shallow_box(&mut self, dst: Place<'tcx>, src: &Operand<'tcx>, _ty: Ty<'tcx>, sp: Span) {
        walk_operand(self, src, sp);
        self.visit_mutate_place(dst, sp);
    }

    #[inline]
    fn visit_assign_constant(&mut self, dst: Place<'tcx>, _src: &ConstOperand<'tcx>, sp: Span) {
        self.visit_mutate_place(dst, sp);
    }

    fn visit_assignment(&mut self, stmt: &(Place<'tcx>, Rvalue<'tcx>), sp: Span) {
        let dst = stmt.0;
        match &stmt.1 {
            Rvalue::Aggregate(kind, ops) => self.visit_assign_aggregate(dst, kind, ops, sp),
            &Rvalue::UnaryOp(op, ref src) => self.visit_assign_unary_op(dst, op, src, sp),
            &Rvalue::Cast(kind, ref src, ty) => self.visit_assign_cast(dst, kind, src, ty, sp),
            &Rvalue::Len(src) => self.visit_assign_len(dst, src, sp),
            &Rvalue::BinaryOp(kind, ref ops) => self.visit_assign_binary_op(dst, kind, ops, sp),
            &Rvalue::Discriminant(src) => self.visit_assign_discriminant(dst, src, sp),
            &Rvalue::CopyForDeref(src) => self.visit_copy_place(dst, src, sp),
            &Rvalue::NullaryOp(ref op, ty) => self.visit_assign_null_op(dst, op, ty, sp),
            &Rvalue::RawPtr(kind, src) => self.visit_assign_raw_ptr(dst, src, kind, sp),
            &Rvalue::Ref(_, kind, src) => self.visit_assign_borrow(dst, src, kind, sp),
            Rvalue::Repeat(value, _) => {
                walk_operand(self, value, sp);
                self.visit_mutate_place(dst, sp);
            },
            &Rvalue::ShallowInitBox(ref src, ty) => self.visit_assign_shallow_box(dst, src, ty, sp),
            Rvalue::Use(src) => match src {
                &Operand::Move(src) => self.visit_move_place(dst, src, sp),
                &Operand::Copy(src) => self.visit_copy_place(dst, src, sp),
                Operand::Constant(src) => self.visit_assign_constant(dst, src, sp),
            },
            Rvalue::ThreadLocalRef(_) => self.visit_mutate_place(dst, sp),
            Rvalue::WrapUnsafeBinder(op, _) => walk_operand(self, op, sp),
        }
    }

    fn visit_copy_nonoverlapping(&mut self, args: &CopyNonOverlapping<'tcx>, sp: Span) {
        walk_operand(self, &args.src, sp);
        walk_operand(self, &args.dst, sp);
        walk_operand(self, &args.count, sp);
        if let Operand::Copy(dst) = args.dst {
            self.visit_mutate_place(dst, sp);
        }
    }

    #[inline]
    fn visit_set_discriminant(&mut self, dst: Place<'tcx>, _variant: VariantIdx, sp: Span) {
        self.visit_mutate_place(dst, sp);
    }

    fn visit_statement(&mut self, stmt: &Statement<'tcx>) {
        let sp = stmt.source_info.span;
        match &stmt.kind {
            StatementKind::Assign(stmt) => self.visit_assignment(stmt, sp),
            &StatementKind::SetDiscriminant {
                ref place,
                variant_index,
            } => self.visit_set_discriminant(**place, variant_index, sp),
            StatementKind::Intrinsic(i) => {
                if let NonDivergingIntrinsic::CopyNonOverlapping(copy) = &**i {
                    self.visit_copy_nonoverlapping(copy, sp);
                }
            },
            &(StatementKind::StorageLive(local) | StatementKind::StorageDead(local)) => {
                // Note: `StorageLive` on a live local fills it with uninit bytes.
                self.visit_uninit_local(local, sp);
            },
            StatementKind::Deinit(place) => self.visit_uninit_place(**place, sp),
            StatementKind::FakeRead(..)
            | StatementKind::Retag(..)
            | StatementKind::PlaceMention(..)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(..)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop
            | StatementKind::BackwardIncompatibleDropHint { .. } => {},
        }
    }

    /// Visits a `Call` terminator.
    ///
    /// By default this will treat calls to `core::mem::drop` the same as a `Drop` terminator.
    #[inline]
    fn visit_call(&mut self, func: &Operand<'tcx>, args: &[Spanned<Operand<'tcx>>], dst: &Place<'tcx>, sp: Span) {
        walk_call(self, func, args, dst, sp);
    }

    /// Visits a `TailCall` terminator.
    ///
    /// By default this will treat calls to `core::mem::drop` the same as a `Drop` terminator.
    #[inline]
    fn visit_tail_call(&mut self, func: &Operand<'tcx>, args: &[Spanned<Operand<'tcx>>], sp: Span) {
        walk_tail_call(self, func, args, sp);
    }

    #[inline]
    fn visit_inline_asm(&mut self, ops: &[InlineAsmOperand<'tcx>], sp: Span) {
        walk_inline_asm(self, ops, sp);
    }

    fn visit_terminator(&mut self, term: &Terminator<'tcx>) {
        let sp = term.source_info.span;
        match &term.kind {
            TerminatorKind::Assert { cond: value, .. } | TerminatorKind::Yield { value, .. } => {
                walk_operand(self, value, sp);
            },
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => self.visit_call(func, args, destination, sp),
            TerminatorKind::TailCall { func, args, .. } => self.visit_tail_call(func, args, sp),
            TerminatorKind::InlineAsm { operands, .. } => self.visit_inline_asm(operands, sp),
            &TerminatorKind::Drop { place, .. } => self.visit_drop_place(place, sp),
            TerminatorKind::Return => self.visit_consume_local(RETURN_PLACE, sp),
            TerminatorKind::SwitchInt { discr, .. } => walk_operand(self, discr, sp),
            TerminatorKind::Goto { .. }
            | TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate { .. }
            | TerminatorKind::Unreachable
            | TerminatorKind::CoroutineDrop
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => {},
        }
    }

    fn visit_block_data(&mut self, block: &BasicBlockData<'tcx>) {
        for stmt in &block.statements {
            self.visit_statement(stmt);
        }
        if let Some(term) = &block.terminator {
            self.visit_terminator(term);
        }
    }
}

pub trait CopyVisitor<'arena, 'tcx, V: Visitor<'arena, 'tcx>>: core::marker::Copy {
    fn copy_range(self, visitor: &mut V, dst: projection::Idx, src: Range<projection::Idx>, sp: Span);
    fn copy_idx(self, visitor: &mut V, dst: projection::Idx, src: projection::Idx, sp: Span);
    fn read_src_range(self, visitor: &mut V, range: Range<projection::Idx>, sp: Span);
    fn read_src_idx(self, visitor: &mut V, idx: projection::Idx, sp: Span);
}

#[derive(Clone, Copy)]
pub struct Move;
impl<'arena, 'tcx, V: Visitor<'arena, 'tcx>> CopyVisitor<'arena, 'tcx, V> for Move {
    fn copy_range(self, visitor: &mut V, dst: projection::Idx, src: Range<projection::Idx>, sp: Span) {
        visitor.visit_move_range(dst, src, sp);
    }
    fn copy_idx(self, visitor: &mut V, dst: projection::Idx, src: projection::Idx, sp: Span) {
        visitor.visit_move_idx(dst, src, sp);
    }
    fn read_src_range(self, visitor: &mut V, range: Range<projection::Idx>, sp: Span) {
        visitor.visit_consume_range(range, sp);
    }
    fn read_src_idx(self, visitor: &mut V, idx: projection::Idx, sp: Span) {
        visitor.visit_consume_idx(idx, sp);
    }
}

#[derive(Clone, Copy)]
pub struct Copy;
impl<'arena, 'tcx, V: Visitor<'arena, 'tcx>> CopyVisitor<'arena, 'tcx, V> for Copy {
    fn copy_range(self, visitor: &mut V, dst: projection::Idx, src: Range<projection::Idx>, sp: Span) {
        visitor.visit_copy_range(dst, src, sp);
    }
    fn copy_idx(self, visitor: &mut V, dst: projection::Idx, src: projection::Idx, sp: Span) {
        visitor.visit_copy_idx(dst, src, sp);
    }
    fn read_src_range(self, visitor: &mut V, range: Range<projection::Idx>, sp: Span) {
        visitor.visit_read_range(range, sp);
    }
    fn read_src_idx(self, visitor: &mut V, idx: projection::Idx, sp: Span) {
        visitor.visit_read_idx(idx, sp);
    }
}

fn copy_aggregate_field<'arena, 'tcx, V: Visitor<'arena, 'tcx>>(
    visitor: &mut V,
    copy_visitor: impl CopyVisitor<'arena, 'tcx, V>,
    dst_start: projection::Idx,
    dst_data: &PlaceData<'_>,
    src: <V::Resolver as Resolver<'arena>>::Resolved,
    sp: Span,
) {
    let (src_start, src_data) = src.values();
    if src_data.contains_values() {
        copy_place(visitor, copy_visitor, dst_start, dst_data, src_start, src_data, sp);
    }
    for idx in src.parents(visitor.resolver()) {
        visitor.visit_read_parent(idx, sp);
    }
}

pub fn copy_place_fields<'arena, 'tcx, V: Visitor<'arena, 'tcx>>(
    visitor: &mut V,
    copy_visitor: impl CopyVisitor<'arena, 'tcx, V>,
    dst: projection::Idx,
    dst_data: &PlaceData<'_>,
    src: projection::Idx,
    src_data: &PlaceData<'_>,
    sp: Span,
) {
    for (dst_field, src_field) in dst_data.fields.iter().zip(src_data.fields) {
        let dst_field_start = dst.plus(dst_field.offset as usize);
        let src_field_start = src.plus(src_field.offset as usize);
        copy_place(
            visitor,
            copy_visitor,
            dst_field_start,
            dst_field.data,
            src_field_start,
            src_field.data,
            sp,
        );
    }
}

pub fn copy_place<'arena, 'tcx, V: Visitor<'arena, 'tcx>>(
    visitor: &mut V,
    copy_visitor: impl CopyVisitor<'arena, 'tcx, V>,
    dst: projection::Idx,
    dst_data: &PlaceData<'_>,
    src: projection::Idx,
    src_data: &PlaceData<'_>,
    sp: Span,
) {
    let src_end = src.plus(src_data.value_count as usize);
    if dst_data == src_data {
        copy_visitor.copy_range(visitor, dst, src..src_end, sp);
    } else if !dst_data.contains_values() {
        copy_visitor.read_src_range(visitor, src..src_end, sp);
    } else if !src_data.contains_values() {
        visitor.visit_mutate_range(dst..dst.plus(dst_data.value_count as usize), sp);
    } else {
        debug_assert_eq!(dst_data.fields.len(), src_data.fields.len());
        match (dst_data.has_value, src_data.has_value) {
            (true, true) => copy_visitor.copy_idx(visitor, dst, src, sp),
            (true, false) => visitor.visit_mutate_idx(dst, sp),
            (false, true) => copy_visitor.read_src_idx(visitor, src, sp),
            (false, false) => {},
        }
        copy_place_fields(visitor, copy_visitor, dst, dst_data, src, src_data, sp);
    }
}

pub fn walk_operand<'tcx>(visitor: &mut impl Visitor<'_, 'tcx>, op: &Operand<'tcx>, sp: Span) {
    match *op {
        Operand::Move(place) => visitor.visit_consume_place(place, sp),
        Operand::Copy(place) => visitor.visit_read_place(place, sp),
        Operand::Constant(_) => {},
    }
}

/// Walks a `Call` terminator.
///
/// This will treat calls to `core::mem::drop` the same as a `Drop` terminator.
pub fn walk_call<'tcx>(
    visitor: &mut impl Visitor<'_, 'tcx>,
    func: &Operand<'tcx>,
    args: &[Spanned<Operand<'tcx>>],
    dst: &Place<'tcx>,
    sp: Span,
) {
    walk_tail_call(visitor, func, args, sp);
    visitor.visit_mutate_place(*dst, sp);
}

/// Walks a `TailCall` terminator.
///
/// This will treat calls to `core::mem::drop` the same as a `Drop` terminator.
pub fn walk_tail_call<'tcx>(
    visitor: &mut impl Visitor<'_, 'tcx>,
    func: &Operand<'tcx>,
    args: &[Spanned<Operand<'tcx>>],
    sp: Span,
) {
    if let [arg] = args
        && let Operand::Move(arg) = arg.node
        && let ty::FnDef(fn_id, _) = *func.ty(visitor.body(), visitor.tcx()).kind()
        && visitor.tcx().is_diagnostic_item(sym::mem_drop, fn_id)
    {
        visitor.visit_drop_place(arg, sp);
    } else {
        walk_operand(visitor, func, sp);
        for arg in args {
            walk_operand(visitor, &arg.node, arg.span);
        }
    }
}

pub fn walk_inline_asm<'tcx>(visitor: &mut impl Visitor<'_, 'tcx>, operands: &[InlineAsmOperand<'tcx>], sp: Span) {
    for op in operands {
        if let InlineAsmOperand::In { value, .. } | InlineAsmOperand::InOut { in_value: value, .. } = op {
            walk_operand(visitor, value, sp);
        }
    }
    for op in operands {
        if let InlineAsmOperand::Out { place: Some(place), .. }
        | InlineAsmOperand::InOut {
            out_place: Some(place), ..
        } = *op
        {
            visitor.visit_mutate_place(place, sp);
        }
    }
}
