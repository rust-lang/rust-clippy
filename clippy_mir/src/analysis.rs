use clippy_data_structures::{SliceSet, bit_slice};
use core::cmp::minmax;
use core::iter;
use rustc_arena::DroplessArena;
use rustc_index::IndexSlice;
use rustc_middle::mir::{BasicBlock, Body, TerminatorKind, UnwindAction};

rustc_index::newtype_index! {
    /// A reordered block index.
    #[orderable]
    pub struct OrderedBlock {}
}

/// Bi-directional mapping to reorder blocks.
pub struct BlockOrderMap<'a> {
    from_ordered: &'a IndexSlice<OrderedBlock, BasicBlock>,
    to_ordered: &'a IndexSlice<BasicBlock, OrderedBlock>,
}
impl<'a> BlockOrderMap<'a> {
    /// Creates a new mapping for a reverse postorder ordering.
    pub fn new_reverse_postorder(arena: &'a DroplessArena, body: &'a Body<'_>) -> Self {
        let from_ordered = IndexSlice::<OrderedBlock, _>::from_raw(body.basic_blocks.reverse_postorder());
        let to_ordered = IndexSlice::<BasicBlock, _>::from_raw_mut(
            arena.alloc_from_iter(iter::repeat_with(|| OrderedBlock::ZERO).take(from_ordered.len())),
        );
        for (x, &y) in from_ordered.iter_enumerated() {
            to_ordered[y] = x;
        }

        Self {
            from_ordered,
            to_ordered,
        }
    }

    #[inline]
    pub fn to_ordered(&self) -> &'a IndexSlice<BasicBlock, OrderedBlock> {
        self.to_ordered
    }

    #[inline]
    pub fn from_ordered(&self) -> &'a IndexSlice<OrderedBlock, BasicBlock> {
        self.from_ordered
    }
}

/// Queue that will remove blocks in-order.
pub struct WorkQueue<'arena> {
    queue: &'arena mut [bit_slice::Word],
    word: bit_slice::Word,
    offset: u32,
    domain_size: u32,
}
impl<'arena> WorkQueue<'arena> {
    /// Creates a new empty queue for the given body.
    pub fn new(arena: &'arena DroplessArena, body: &Body<'_>) -> Self {
        Self {
            queue: arena.alloc_from_iter(iter::repeat_n(
                0,
                bit_slice::word_count_from_bits(body.basic_blocks.len()),
            )),
            offset: 0,
            word: 0,
            domain_size: body.basic_blocks.len() as u32,
        }
    }

    /// Fills the queue with all blocks.
    fn fill(&mut self) {
        self.queue.fill(!0);
        if let Some(word) = self.queue.last_mut() {
            *word &= bit_slice::final_mask_for_size(self.domain_size as usize);
        }
        self.offset = 0;
        self.word = self.queue.first().copied().unwrap_or(0);
    }

    /// Extracts the next block in the queue.
    fn next(&mut self) -> Option<OrderedBlock> {
        if self.word == 0 {
            self.queue[self.offset as usize] = 0;
            self.offset += self.queue[self.offset as usize + 1..].iter().position(|&x| x != 0)? as u32 + 1;
            self.word = self.queue[self.offset as usize];
        }
        let bit = self.word.trailing_zeros() as usize;
        self.word ^= 1 << bit;
        Some(OrderedBlock::from_usize(
            bit | self.offset as usize * bit_slice::WORD_BITS,
        ))
    }

    /// Inserts a single block into the queue.
    #[track_caller]
    pub fn insert(&mut self, block: OrderedBlock) {
        debug_assert!(block.as_u32() < self.domain_size);
        let word = block.as_usize() / bit_slice::WORD_BITS;
        let bit = 1 << block.as_usize() % bit_slice::WORD_BITS;

        self.queue[self.offset as usize] = self.word;
        self.queue[word] |= bit;
        self.offset = self.offset.min(word as u32);
        self.word |= self.queue[self.offset as usize];
    }

    /// Inserts a sorted sequence of blocks into the queue.
    #[track_caller]
    pub fn insert_sorted(&mut self, blocks: impl IntoIterator<Item = OrderedBlock>) {
        let mut blocks = blocks.into_iter();
        let Some(block) = blocks.next() else {
            return;
        };
        debug_assert!(block.as_u32() < self.domain_size);
        let block = block.as_usize();
        let word = block / bit_slice::WORD_BITS;
        let bit = 1 << block % bit_slice::WORD_BITS;

        self.queue[self.offset as usize] = self.word;
        self.offset = self.offset.min(word as u32);

        self.queue[word] |= bit;
        for block in blocks {
            debug_assert!(block.as_u32() < self.domain_size);
            let idx = block.as_usize() / bit_slice::WORD_BITS;
            let bit = 1 << block.as_usize() % bit_slice::WORD_BITS;
            self.queue[idx] |= bit;
        }

        self.word = self.queue[self.offset as usize];
    }
}

/// Extracts the body's edges and orders them via the block map.
pub fn get_body_edges<'arena>(
    arena: &'arena DroplessArena,
    body: &Body<'_>,
    block_map: &BlockOrderMap<'_>,
) -> &'arena IndexSlice<OrderedBlock, &'arena SliceSet<OrderedBlock>> {
    let blocks = IndexSlice::<OrderedBlock, _>::from_raw_mut(
        arena.alloc_from_iter(iter::repeat(SliceSet::empty()).take(body.basic_blocks.len())),
    );
    for (block, block_data) in body.basic_blocks.iter_enumerated() {
        blocks[block_map.to_ordered[block]] = match block_data.terminator().kind {
            TerminatorKind::Drop {
                target,
                unwind: UnwindAction::Cleanup(cleanup),
                ..
            }
            | TerminatorKind::Call {
                target: Some(target),
                unwind: UnwindAction::Cleanup(cleanup),
                ..
            }
            | TerminatorKind::Assert {
                target,
                unwind: UnwindAction::Cleanup(cleanup),
                ..
            }
            | TerminatorKind::Yield {
                resume: target,
                drop: Some(cleanup),
                ..
            } => SliceSet::from_sorted(
                arena.alloc_from_iter(minmax(block_map.to_ordered[target], block_map.to_ordered[cleanup])),
            ),

            TerminatorKind::Goto { target }
            | TerminatorKind::Drop { target, .. }
            | TerminatorKind::Assert { target, .. }
            | TerminatorKind::Call {
                target: Some(target), ..
            }
            | TerminatorKind::Call {
                unwind: UnwindAction::Cleanup(target),
                ..
            }
            | TerminatorKind::Yield { resume: target, .. }
            | TerminatorKind::FalseEdge {
                real_target: target, ..
            }
            | TerminatorKind::FalseUnwind {
                real_target: target, ..
            } => SliceSet::from_ref(arena.alloc(block_map.to_ordered[target])),

            TerminatorKind::SwitchInt { ref targets, .. } => SliceSet::from_unsorted_slice_dedup(
                arena.alloc_from_iter(targets.all_targets().iter().map(|&target| block_map.to_ordered[target])),
            ),

            TerminatorKind::InlineAsm {
                ref targets, unwind, ..
            } => {
                let targets = targets.iter().map(|&target| block_map.to_ordered[target]);
                SliceSet::from_unsorted_slice(if let UnwindAction::Cleanup(cleanup) = unwind {
                    arena.alloc_from_iter(targets.chain([block_map.to_ordered[cleanup]]))
                } else {
                    arena.alloc_from_iter(targets)
                })
            },

            TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::TailCall { .. }
            | TerminatorKind::Call { .. }
            | TerminatorKind::CoroutineDrop => SliceSet::empty(),
        }
    }
    blocks
}

pub trait Analysis {
    type Domain;
    fn clone_block_entry(&mut self, src: &Self::Domain, dst: &mut Self::Domain, block: OrderedBlock);
    fn join_domain(
        &mut self,
        src: &Self::Domain,
        dst: &mut Self::Domain,
        src_block: OrderedBlock,
        dst_block: OrderedBlock,
    ) -> bool;
    fn apply_block(&mut self, state: &mut Self::Domain, block: OrderedBlock);
}

pub fn run_analysis<A: Analysis>(
    queue: &mut WorkQueue,
    edges: &IndexSlice<OrderedBlock, &SliceSet<OrderedBlock>>,
    states: &mut IndexSlice<OrderedBlock, A::Domain>,
    tmp_state: &mut A::Domain,
    analysis: &mut A,
) {
    debug_assert_eq!(queue.domain_size as usize, edges.len());
    debug_assert_eq!(queue.domain_size as usize, states.len());

    queue.fill();
    while let Some(block) = queue.next() {
        analysis.clone_block_entry(&states[block], tmp_state, block);
        analysis.apply_block(tmp_state, block);
        queue.insert_sorted(
            edges[block]
                .iter()
                .copied()
                .filter(|&dst_block| analysis.join_domain(tmp_state, &mut states[dst_block], block, dst_block)),
        );
    }
}
