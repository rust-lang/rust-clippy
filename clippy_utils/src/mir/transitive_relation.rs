use rustc_ast::Mutability;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::mir;

#[derive(Debug, Default)]
pub(super) struct TransitiveRelation {
    relations: FxHashMap<mir::Local, Vec<(mir::Local, Mutability)>>,
}

impl TransitiveRelation {
    pub fn add(&mut self, a: mir::Local, b: mir::Local, mutability: Mutability) {
        self.relations.entry(a).or_default().push((b, mutability));
    }

    pub fn reachable_from(&self, a: mir::Local, domain_size: usize) -> FxHashMap<mir::Local, Mutability> {
        let mut seen = FxHashMap::default();
        seen.reserve(domain_size);
        let mut stack = vec![(a, Mutability::Not)];
        while let Some((u, u_mut)) = stack.pop() {
            if let Some(edges) = self.relations.get(&u) {
                for &(v, v_mut) in edges {
                    let new_mut = Mutability::max(u_mut, v_mut);
                    if seen.insert(v, new_mut).is_none() {
                        stack.push((v, new_mut));
                    }
                }
            }
        }
        seen
    }
}
