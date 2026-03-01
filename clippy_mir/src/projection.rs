//! Create mappings that can resolve local places to a set of tracked values.
//!
//! Starting with each local as a tree where the local is the root node, each field is a
//! child node, and sub-fields are children of their respective nodes; a projection mapping
//! will map each node to a unique index. Once constructed this mapping can be used to
//! resolve a place to it's matching value and that values child and parent values.
//!
//! The constructed map may have multiple filters which prevent nodes from being given an
//! associated index:
//!
//! * First is a visibility filter. Any field which can not be accessed from the current body will
//!   not be assigned an index. This filter is not optional.
//! * Second is a type based filter. This will prevent certain types from being assigned an index,
//!   but will still allow both parents and children to be given one.
//! * Third is a place based filter. This will prevent a specific place as well as both it's parents
//!   and children from being assigned an index.
//!
//! # Example
//!
//! Given the following struct:
//!
//! ```rust
//! struct Foo {
//!     x: u32,
//!     y: (u32, i32),
//! }
//! ```
//!
//! This will create the following tree (each node's index is in parenthesis):
//!
//! ```none
//!      Foo (0)
//!     /       \
//!   x (1)    y (2)
//!           /     \
//!         0 (3)  1 (4)
//! ```
//!
//! Places within the struct are resolved as follows:
//!
//! * Foo:
//!   * parents: N/A
//!   * values: 0, 1, 2, 3, 4
//! * Foo.x:
//!   * parents: 0
//!   * values: 1
//! * Foo.y:
//!   * parents: 0
//!   * values: 2, 3, 4
//! * Foo.y.0:
//!   * parents: 2, 0
//!   * values: 3
//! * Foo.y.1:
//!   * parents: 2, 0
//!   * values: 4
//!
//! If tuples were filtered from storing a value the following tree would be constructed:
//!
//! ```none
//!      Foo (0)
//!     /       \
//!   x (1)      y
//!           /     \
//!         0 (2)  1 (3)
//! ```
//!
//! Places would be resolved as follows:
//!
//! * Foo:
//!   * parents: N/A
//!   * values: 0, 1, 2, 3
//! * Foo.x:
//!   * parents: 0
//!   * values: 1
//! * Foo.y:
//!   * parents: 0
//!   * values: 2, 3
//! * Foo.y.0:
//!   * parents: 0
//!   * values: 3
//! * Foo.y.1:
//!   * parents: 0
//!   * values: 4

use clippy_data_structures::CountedIter;
use core::ops::Range;
use core::{ptr, slice};
use rustc_abi::FieldIdx;
use rustc_arena::DroplessArena;
use rustc_data_structures::fx::FxHashMap;
use rustc_index::{Idx as _, IndexSlice};
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{Body, Local, Location, Place, ProjectionElem, Rvalue};
use rustc_middle::ty::{Ty, TyCtxt, TyKind, TypingEnv};
use rustc_span::def_id::DefId;

rustc_index::newtype_index! {
    /// Index to a value
    #[orderable]
    pub struct Idx {}
}

#[derive(Clone, Copy)]
pub struct FieldData<'arena> {
    /// The offset to use to get to the first value stored for this field.
    pub offset: u32,
    /// The place data for this field.
    pub data: &'arena PlaceData<'arena>,
}
impl FieldData<'_> {
    /// A field with no values.
    pub const EMPTY: Self = Self {
        // The offset doesn't actually matter since the occupied range is empty.
        offset: 0,
        data: EMPTY_PLACE_DATA,
    };
}

/// Traversal data about a node in the projection tree.
#[non_exhaustive]
pub struct PlaceData<'arena> {
    /// The offset and place data for each immediate child.
    pub fields: &'arena IndexSlice<FieldIdx, FieldData<'arena>>,
    /// The number of values stored by this place and it's children.
    pub value_count: u32,
    /// Is a value stored for this place.
    pub has_value: bool,
}

// Avoid the need to allocate the two most common values.
pub static EMPTY_PLACE_DATA: &PlaceData<'_> = &PlaceData {
    fields: IndexSlice::from_raw(&[]),
    value_count: 0,
    has_value: false,
};
pub static SINGLE_PLACE_DATA: &PlaceData<'_> = &PlaceData {
    fields: IndexSlice::from_raw(&[]),
    value_count: 1,
    has_value: true,
};

impl PartialEq for PlaceData<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Most instances will be interned so use pointer equality here.
        ptr::addr_eq(self, other)
    }
}

impl<'arena> PlaceData<'arena> {
    #[inline]
    #[must_use]
    pub fn contains_values(&self) -> bool {
        // No need to dereference. All empty instances are replaced with `EMPTY_PLACE_DATA`.
        self != EMPTY_PLACE_DATA
    }

    pub fn alloc_new(
        arena: &'arena DroplessArena,
        has_value: bool,
        fields: impl ExactSizeIterator<Item = &'arena Self>,
    ) -> &'arena Self {
        let mut value_count = u32::from(has_value);
        let fields = arena.alloc_from_iter(fields.map(|data| {
            let offset = value_count;
            value_count += data.value_count;
            FieldData { offset, data }
        }));
        if value_count == u32::from(has_value) {
            if has_value { SINGLE_PLACE_DATA } else { EMPTY_PLACE_DATA }
        } else {
            arena.alloc(Self {
                fields: IndexSlice::from_raw(fields),
                value_count,
                has_value,
            })
        }
    }
}

/// Type-based interner for `ProjectionData`
struct TyProjectionInterner<'arena, 'tcx, F> {
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    arena: &'arena DroplessArena,
    ty_has_value: F,
    vis_filter: DefId,
    projection_data: FxHashMap<Ty<'tcx>, &'arena PlaceData<'arena>>,
}
impl<'arena, 'tcx, F> TyProjectionInterner<'arena, 'tcx, F>
where
    F: FnMut(Ty<'tcx>) -> bool,
{
    fn new(
        tcx: TyCtxt<'tcx>,
        typing_env: TypingEnv<'tcx>,
        arena: &'arena DroplessArena,
        ty_has_value: F,
        vis_filter: DefId,
    ) -> Self {
        Self {
            tcx,
            typing_env,
            arena,
            ty_has_value,
            vis_filter,
            projection_data: FxHashMap::default(),
        }
    }

    /// Creates a new `ProjectionData` for the given type.
    fn alloc_for_ty(&mut self, ty: Ty<'tcx>) -> &'arena PlaceData<'arena> {
        let has_value = (self.ty_has_value)(ty);
        match *ty.kind() {
            TyKind::Adt(def, args) if def.is_struct() => PlaceData::alloc_new(
                self.arena,
                has_value,
                def.non_enum_variant().fields.iter().map(|f| {
                    if f.vis.is_accessible_from(self.vis_filter, self.tcx) {
                        let ty = f.ty(self.tcx, args);
                        self.intern(
                            self.tcx
                                .try_normalize_erasing_regions(self.typing_env, ty)
                                .unwrap_or(ty),
                        )
                    } else {
                        EMPTY_PLACE_DATA
                    }
                }),
            ),
            TyKind::Tuple(tys) => PlaceData::alloc_new(self.arena, has_value, tys.iter().map(|ty| self.intern(ty))),
            _ if has_value => SINGLE_PLACE_DATA,
            _ => EMPTY_PLACE_DATA,
        }
    }

    /// Interns the `ProjectionData` for the given type.
    fn intern(&mut self, ty: Ty<'tcx>) -> &'arena PlaceData<'arena> {
        if let Some(&data) = self.projection_data.get(&ty) {
            data
        } else {
            let data = self.alloc_for_ty(ty);
            self.projection_data.insert(ty, data);
            data
        }
    }
}

pub(crate) struct PlaceFilterIter<'a> {
    iter: slice::Iter<'a, LocalPlace<'a>>,
    pub local: Option<Local>,
    pub projection: &'a [FieldIdx],
}
impl PlaceFilterIter<'_> {
    /// Creates a new `ProjectionData` by applying the current filter.
    ///
    /// This will move to the next filter not affecting the current field.
    pub(crate) fn apply_current<'arena>(
        &mut self,
        arena: &'arena DroplessArena,
        fields: &'arena IndexSlice<FieldIdx, FieldData<'arena>>,
        depth: usize,
    ) -> &'arena PlaceData<'arena> {
        if let Some(&filter_field) = self.projection.get(depth) {
            let filter_local = self.local;
            let filter_projection = &self.projection[..depth];
            let mut filter_field: Option<FieldIdx> = Some(filter_field);
            let data = PlaceData::alloc_new(
                arena,
                false,
                fields.iter_enumerated().map(|(field, field_data)| {
                    if filter_field == Some(field) {
                        let fields = field_data.data.fields;
                        let data = self.apply_current(arena, fields, depth + 1);
                        // Get the next field to filter if the filter still has the same parent field.
                        filter_field =
                            self.projection.get(depth).copied().filter(|_| {
                                self.local == filter_local && self.projection.starts_with(filter_projection)
                            });
                        data
                    } else {
                        field_data.data
                    }
                }),
            );
            // Skip to the filter after the current field.
            // Note: Child fields may have been dropped before applying this filter.
            while filter_field.is_some() {
                (self.local, self.projection) = self
                    .iter
                    .next()
                    .map_or((None, [].as_slice()), |x| (Some(x.local), x.projection));
                filter_field = self
                    .projection
                    .get(depth)
                    .copied()
                    .filter(|_| self.local == filter_local && self.projection.starts_with(filter_projection));
            }
            data
        } else {
            // Found the filtered field. Step to the next filter.
            (self.local, self.projection) = self
                .iter
                .next()
                .map_or((None, [].as_slice()), |x| (Some(x.local), x.projection));
            EMPTY_PLACE_DATA
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LocalPlace<'arena> {
    local: Local,
    projection: &'arena [FieldIdx],
}
impl<'arena> LocalPlace<'arena> {
    fn from_place(arena: &'arena DroplessArena, place: Place<'_>) -> Self {
        Self {
            local: place.local,
            projection: arena.alloc_from_iter(CountedIter(place.projection.iter().map_while(|proj| {
                if let ProjectionElem::Field(idx, _) = proj {
                    Some(idx)
                } else {
                    None
                }
            }))),
        }
    }

    fn is_parent_of(self, other: LocalPlace) -> bool {
        self.local == other.local
            && self.projection.len() <= other.projection.len()
            && self.projection.iter().zip(other.projection).all(|(&x, &y)| x == y)
    }
}

pub struct PlaceFilter<'a> {
    filter: Vec<LocalPlace<'a>>,
}
impl<'a> PlaceFilter<'a> {
    /// Creates a filter which will remove all places that have a raw borrow taken.
    pub fn new_raw_borrow_filter(arena: &'a DroplessArena, body: &Body<'_>) -> Self {
        struct V<'a> {
            arena: &'a DroplessArena,
            borrows: Vec<LocalPlace<'a>>,
        }
        impl<'tcx> Visitor<'tcx> for V<'_> {
            fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, _: Location) {
                if let Rvalue::RawPtr(_, place) = *rvalue {
                    self.borrows.push(LocalPlace::from_place(self.arena, place));
                }
            }
        }
        let mut v = V {
            arena,
            borrows: Vec::new(),
        };
        for (block, block_data) in body.basic_blocks.iter_enumerated() {
            v.visit_basic_block_data(block, block_data);
        }
        v.borrows.sort();
        // Remove sub-field filters when the parent field is also filtered.
        // Not doing so will break the filtering algorithm.
        v.borrows.dedup_by(|&mut second, &mut first| first.is_parent_of(second));
        Self { filter: v.borrows }
    }

    #[expect(clippy::iter_not_returning_iterator)]
    pub(crate) fn iter(&self) -> PlaceFilterIter<'_> {
        let mut iter = self.filter.iter();
        let (local, projection) = iter
            .next()
            .map_or((None, [].as_slice()), |x| (Some(x.local), x.projection));
        PlaceFilterIter {
            iter,
            local,
            projection,
        }
    }
}

#[derive(Clone)]
struct ResolvedParentsField<'a> {
    fields: slice::Iter<'a, FieldData<'a>>,
    /// The base index to use for all fields.
    idx: Idx,
    /// The parent index to use for all fields.
    parent: Option<Idx>,
}
struct ResolvedParents<'a> {
    locals: slice::Iter<'a, (Idx, &'a PlaceData<'a>)>,
    parents: Vec<ResolvedParentsField<'a>>,
    current: ResolvedParentsField<'a>,
    hint: u32,
}
impl<'a> ResolvedParents<'a> {
    fn new(locals: &'a IndexSlice<Local, (Idx, &'a PlaceData<'a>)>, hint: u32) -> Self {
        Self {
            locals: locals.iter(),
            parents: Vec::new(),
            current: ResolvedParentsField {
                fields: [].iter(),
                idx: Idx::ZERO,
                parent: None,
            },
            hint,
        }
    }
}
impl Iterator for ResolvedParents<'_> {
    type Item = Option<Idx>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(field) = self.current.fields.next() {
                self.parents.push(self.current.clone());
                let parent = self.current.parent;
                self.current = ResolvedParentsField {
                    fields: field.data.fields.iter(),
                    idx: self.current.idx.plus(field.offset as usize),
                    parent: None,
                };
                if field.data.has_value {
                    self.current.parent = Some(self.current.idx);
                    return Some(parent);
                }
            } else if let Some(field) = self.parents.pop() {
                self.current = field;
            } else {
                let &(idx, projection) = self.locals.by_ref().find(|&(_, data)| data.contains_values())?;
                self.current = ResolvedParentsField {
                    fields: projection.fields.iter(),
                    idx,
                    parent: self.parents.last().and_then(|x| x.parent),
                };
                if projection.has_value {
                    self.current.parent = Some(self.current.idx);
                    return Some(None);
                }
            }
        }
    }

    /// Pass the size to `DroplessArena::alloc_from_iter`
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.hint as usize, Some(self.hint as usize))
    }
}

/// A place which has been resolved by a projection map.
pub trait ResolvedPlace<'arena>: Copy {
    type Resolver;
    type Parents: Iterator<Item = Idx>;

    /// Gets the first value index and the projection data for the place.
    fn values(&self) -> (Idx, &'arena PlaceData<'arena>);

    /// whether the place involve a deref projection.
    fn is_deref(&self) -> bool;

    /// The parents of the place from most to least specific.
    fn parents(&self, map: &Self::Resolver) -> Self::Parents;

    // Checks if this place affects any values.
    fn affects_any_value(&self) -> bool;

    /// Gets the contained value assuming the place refers to a scalar value.
    ///
    /// # Panics
    /// This may panic if this place contains multiple values.
    fn as_scalar_value(self) -> Option<Idx>;
}

pub trait Resolver<'arena> {
    type Resolved: ResolvedPlace<'arena, Resolver = Self>;

    /// Resolves the place to the set of values it contains.
    fn resolve(&self, place: Place<'_>) -> Self::Resolved;
    /// Resolves the local to the set of values it contains.
    fn resolve_local(&self, local: Local) -> (Idx, &'arena PlaceData<'arena>);

    /// Gets the set of values contained in the body's arguments.
    fn resolve_args(&self, body: &Body<'_>) -> Range<Idx> {
        if body.arg_count > 0 {
            let (args_start, _) = self.resolve_local(Local::from_u32(1));
            let (args_end, args_data) = self.resolve_local(Local::from_usize(1 + body.arg_count));
            args_start..args_end.plus(args_data.value_count as usize)
        } else {
            Idx::ZERO..Idx::ZERO
        }
    }
}

#[derive(Clone)]
pub struct ParentIter<'a> {
    parent_map: &'a IndexSlice<Idx, Option<Idx>>,
    next: Option<Idx>,
}
impl Iterator for ParentIter<'_> {
    type Item = Idx;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            Some(x) => {
                self.next = self.parent_map[x];
                Some(x)
            },
            None => None,
        }
    }
}

/// A place which has been resolved by a projection map.
#[derive(Clone, Copy)]
pub enum Resolved<'arena> {
    Value {
        data: &'arena PlaceData<'arena>,
        parent: Option<Idx>,
        idx: Idx,
    },
    Deref {
        parent: Idx,
    },
}
impl<'arena> ResolvedPlace<'arena> for Resolved<'arena> {
    type Resolver = Map<'arena>;
    type Parents = ParentIter<'arena>;

    #[inline]
    fn values(&self) -> (Idx, &'arena PlaceData<'arena>) {
        if let Self::Value { data, idx, .. } = *self {
            (idx, data)
        } else {
            (Idx::ZERO, EMPTY_PLACE_DATA)
        }
    }

    #[inline]
    fn is_deref(&self) -> bool {
        matches!(self, Self::Deref { .. })
    }

    #[inline]
    fn parents(&self, map: &Map<'arena>) -> Self::Parents {
        ParentIter {
            parent_map: map.parent_map,
            next: match *self {
                Self::Value { parent, .. } => parent,
                Self::Deref { parent } => Some(parent),
            },
        }
    }

    #[inline]
    fn affects_any_value(&self) -> bool {
        if let Self::Value { data, parent, .. } = *self {
            data.contains_values() || parent.is_some()
        } else {
            true
        }
    }

    #[inline]
    fn as_scalar_value(self) -> Option<Idx> {
        match self {
            Self::Value { data, idx, .. } => {
                debug_assert_eq!(data.value_count, u32::from(data.has_value));
                data.has_value.then_some(idx)
            },
            Self::Deref { .. } => None,
        }
    }
}

/// Mapping between local projections and the range of values they occupy.
pub struct Map<'arena> {
    local_map: &'arena IndexSlice<Local, (Idx, &'arena PlaceData<'arena>)>,
    parent_map: &'arena IndexSlice<Idx, Option<Idx>>,
}
impl<'arena> Map<'arena> {
    pub fn new<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: TypingEnv<'tcx>,
        arena: &'arena DroplessArena,
        body: &Body<'tcx>,
        ty_has_value: impl FnMut(Ty<'tcx>) -> bool,
        vis_filter: DefId,
        place_filter: &PlaceFilter<'_>,
    ) -> Self {
        let mut interner = TyProjectionInterner::new(tcx, typing_env, arena, ty_has_value, vis_filter);
        let mut idx_count: u32 = 0u32;
        let mut place_filter = place_filter.iter();
        let local_map = IndexSlice::<Local, _>::from_raw(arena.alloc_from_iter(
            body.local_decls.iter_enumerated().map(|(local, local_decl)| {
                let data = interner.intern(
                    tcx.try_normalize_erasing_regions(typing_env, local_decl.ty)
                        .unwrap_or(local_decl.ty),
                );
                let data = if place_filter.local.is_some_and(|filter| filter == local) {
                    place_filter.apply_current(arena, data.fields, 0)
                } else {
                    data
                };
                let idx = idx_count;
                idx_count += data.value_count;
                (Idx::from_u32(idx), data)
            }),
        ));
        let parent_map =
            IndexSlice::<Idx, _>::from_raw(arena.alloc_from_iter(ResolvedParents::new(local_map, idx_count)));
        Self { local_map, parent_map }
    }

    /// Gets the number of values
    #[must_use]
    pub fn domain_size(&self) -> usize {
        self.parent_map.len()
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn domain_size_u32(&self) -> u32 {
        self.parent_map.len() as u32
    }

    #[must_use]
    pub fn local_for_idx(&self, idx: Idx) -> Local {
        let mut res = Local::ZERO;
        for (l, &(x, data)) in self.local_map.iter_enumerated() {
            if data.has_value {
                if x <= idx {
                    res = l;
                } else {
                    break;
                }
            }
        }
        res
    }
}
impl<'arena> Resolver<'arena> for Map<'arena> {
    type Resolved = Resolved<'arena>;

    fn resolve(&self, place: Place) -> Self::Resolved {
        let (mut idx, mut data) = self.local_map[place.local];
        let mut parent = None;
        let mut projections = place.projection.iter();
        while let Some(projection) = projections.next() {
            if data.has_value {
                parent = Some(idx);
            }
            if let ProjectionElem::Field(field, _) = projection {
                // Note: if all fields contain no value then no field data will be stored.
                if let Some(field) = data.fields.get(field) {
                    data = field.data;
                    idx = idx.plus(field.offset as usize);
                    continue;
                }
                data = EMPTY_PLACE_DATA;
            }
            if let Some(parent) = parent
                && (matches!(projection, ProjectionElem::Deref)
                    || projections.any(|projection| matches!(projection, ProjectionElem::Deref)))
            {
                return Resolved::Deref { parent };
            }
            // At this point we either have a deref of an untracked value, or a projection
            // that stays within the local.
            break;
        }
        Resolved::Value { data, parent, idx }
    }

    fn resolve_local(&self, local: Local) -> (Idx, &'arena PlaceData<'arena>) {
        self.local_map[local]
    }
}
