use std::marker::PhantomData;

use super::btree_traits::BTreeParams;

pub struct TreeStats<T: BTreeParams> {
    _phantom: PhantomData<T>,
    pub size: usize,
    pub leaves: usize,
    pub inner_nodes: usize,
}

impl<T: BTreeParams> TreeStats<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
            size: 0,
            leaves: 0,
            inner_nodes: 0,
        }
    }

    pub fn nodes(&self) -> usize {
        self.inner_nodes + self.leaves
    }

    pub fn avgfill_leaves(&self) -> f64 {
        self.size as f64 / (self.leaves * T::LEAF_SLOTMAX) as f64
    }
}
