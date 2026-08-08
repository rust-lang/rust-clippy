use core::option;
use rustc_arena::DroplessArena;
use rustc_data_structures::fx::FxHashMap;
use rustc_index::{Idx as _, IndexSlice};
use rustc_middle::mir::{Body, Local, Place, PlaceElem, ProjectionElem};
use rustc_middle::ty::{Ty, TyCtxt, TyKind, TypingEnv};
use rustc_span::def_id::DefId;

pub use crate::projection::{
    EMPTY_PLACE_DATA, Idx, PlaceData, PlaceFilter, ResolvedPlace, Resolver, SINGLE_PLACE_DATA,
};

/// Type-based interner for `ProjectionData`.
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
        if (self.ty_has_value)(ty) {
            SINGLE_PLACE_DATA
        } else {
            match *ty.kind() {
                TyKind::Adt(def, args) if def.is_struct() => PlaceData::alloc_new(
                    self.arena,
                    false,
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
                TyKind::Tuple(tys) => PlaceData::alloc_new(self.arena, false, tys.iter().map(|ty| self.intern(ty))),
                _ => EMPTY_PLACE_DATA,
            }
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

/// A resolved place according to a childless projection map.
#[derive(Clone, Copy)]
pub enum Resolved<'arena> {
    Value {
        start: Idx,
        data: &'arena PlaceData<'arena>,
    },
    Child {
        parent: Idx,
    },
    Deref {
        parent: Idx,
    },
}
impl<'arena> ResolvedPlace<'arena> for Resolved<'arena> {
    type Resolver = Map<'arena>;
    type Parents = option::IntoIter<Idx>;

    #[inline]
    fn values(&self) -> (Idx, &'arena PlaceData<'arena>) {
        if let Self::Value { start, data } = *self {
            (start, data)
        } else {
            (Idx::ZERO, EMPTY_PLACE_DATA)
        }
    }

    #[inline]
    fn is_deref(&self) -> bool {
        matches!(self, Self::Deref { .. })
    }

    #[inline]
    fn parents(&self, _: &Map<'arena>) -> Self::Parents {
        if let Self::Deref { parent } | Self::Child { parent } = *self {
            Some(parent).into_iter()
        } else {
            None.into_iter()
        }
    }

    #[inline]
    fn affects_any_value(&self) -> bool {
        if let Self::Value { data, .. } = *self {
            data.contains_values()
        } else {
            true
        }
    }

    #[inline]
    fn as_scalar_value(self) -> Option<Idx> {
        if let Self::Value { data, start } = self
            && data.contains_values()
        {
            debug_assert_eq!(data.value_count, 1);
            debug_assert!(data.has_value);
            Some(start)
        } else {
            None
        }
    }
}

/// Mapping between local projections and the range of values they occupy.
///
/// Like `Map`, but each place containing a value will not have any child nodes.
pub struct Map<'arena> {
    local_map: &'arena IndexSlice<Local, (Idx, &'arena PlaceData<'arena>)>,
    domain_size: u32,
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
        Self {
            local_map: IndexSlice::from_raw(arena.alloc_from_iter(body.local_decls.iter_enumerated().map(
                |(local, local_decl)| {
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
                },
            ))),
            domain_size: idx_count,
        }
    }

    /// Gets the number of values
    #[must_use]
    pub fn domain_size(&self) -> usize {
        self.domain_size as usize
    }

    #[must_use]
    pub fn resolve_slice_proj(&self, local: Local, projection: &[PlaceElem<'_>]) -> Resolved<'arena> {
        let (mut idx, mut data) = self.local_map[local];
        let mut projections = projection.iter();
        while !data.has_value {
            if let Some(projection) = projections.next()
                && let &ProjectionElem::Field(field, _) = projection
            {
                // Note: if all fields contain no value then `data.fields` will be empty.
                if let Some(field) = data.fields.get(field) {
                    data = field.data;
                    idx = idx.plus(field.offset as usize);
                    continue;
                }
                data = EMPTY_PLACE_DATA;
            }
            break;
        }
        if data.has_value {
            if projections
                .clone()
                .any(|projection| matches!(projection, ProjectionElem::Deref))
            {
                return Resolved::Deref { parent: idx };
            } else if projections
                .next()
                .is_some_and(|projection| matches!(projection, ProjectionElem::Field(..)))
            {
                return Resolved::Child { parent: idx };
            }
        }
        Resolved::Value { data, start: idx }
    }
}
impl<'arena> Resolver<'arena> for Map<'arena> {
    type Resolved = Resolved<'arena>;

    fn resolve_local(&self, local: Local) -> (Idx, &'arena PlaceData<'arena>) {
        self.local_map[local]
    }

    fn resolve(&self, place: Place<'_>) -> Self::Resolved {
        self.resolve_slice_proj(place.local, place.projection)
    }
}
