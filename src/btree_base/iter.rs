use std::fmt::Debug;

use super::{
    btree_traits::BTreeParams,
    node::{LeafNode, NodeImpl},
};

pub trait TreeIterator<T: BTreeParams>: Clone + Debug {
    fn key(&self) -> &T::KeyType;
    fn forward(&mut self) -> &mut Self;
    fn backward(&mut self) -> &mut Self;
    fn equals(&self, other: &Self) -> bool;
    fn value(&self) -> &T::ValueType;
}

#[derive(Clone, Debug)]
pub struct BTreeIterator<T: BTreeParams>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub curr_leaf: *mut LeafNode<T>,
    pub curr_slot: u16,
}

impl<T: BTreeParams> BTreeIterator<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub fn new(leaf: *mut LeafNode<T>, slot: u16) -> Self {
        Self {
            curr_leaf: leaf,
            curr_slot: slot,
        }
    }

    #[inline]
    fn curr_leaf(&self) -> &LeafNode<T> {
        unsafe { &*self.curr_leaf }
    }
}

impl<T: BTreeParams> TreeIterator<T> for BTreeIterator<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    #[inline]
    fn key(&self) -> &T::KeyType {
        &self.curr_leaf().key(self.curr_slot as usize)
    }

    fn forward(&mut self) -> &mut Self {
        if self.curr_slot + 1 < self.curr_leaf().as_node().slotuse {
            self.curr_slot += 1;
        } else if !self.curr_leaf().next_leaf.is_null() {
            self.curr_leaf = self.curr_leaf().next_leaf;
            self.curr_slot = 0;
        } else {
            self.curr_slot = self.curr_leaf().as_node().slotuse;
        }

        self
    }

    fn backward(&mut self) -> &mut Self {
        if self.curr_slot > 0 {
            self.curr_slot -= 1;
        } else if !self.curr_leaf().prev_leaf.is_null() {
            self.curr_leaf = self.curr_leaf().prev_leaf;
            self.curr_slot = self.curr_leaf().as_node().slotuse - 1;
        } else {
            self.curr_slot = 0;
        }

        self
    }

    fn equals(&self, other: &Self) -> bool {
        self.curr_leaf == other.curr_leaf && self.curr_slot == other.curr_slot
    }

    fn value(&self) -> &T::ValueType {
        unsafe {
            (*self.curr_leaf).slotdata[self.curr_slot as usize]
                .assume_init_ref()
                .value()
        }
    }
}

#[derive(Clone, Debug)]
pub struct BTreeReverseIterator<T: BTreeParams>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub curr_leaf: *mut LeafNode<T>,
    pub curr_slot: u16,
}

impl<T: BTreeParams> BTreeReverseIterator<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub fn new(leaf: *mut LeafNode<T>, slot: u16) -> Self {
        Self {
            curr_leaf: leaf,
            curr_slot: slot,
        }
    }
    fn curr_leaf(&self) -> &LeafNode<T> {
        unsafe { &*self.curr_leaf }
    }
}

impl<T: BTreeParams> TreeIterator<T> for BTreeReverseIterator<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn key(&self) -> &T::KeyType {
        &self.curr_leaf().key(self.curr_slot as usize - 1)
    }

    fn forward(&mut self) -> &mut Self {
        if self.curr_slot > 1 {
            self.curr_slot -= 1;
        } else if !self.curr_leaf().prev_leaf.is_null() {
            self.curr_leaf = self.curr_leaf().prev_leaf;
            self.curr_slot = self.curr_leaf().as_node().slotuse;
        } else {
            self.curr_slot = 0;
        }

        self
    }

    fn backward(&mut self) -> &mut Self {
        if self.curr_slot < self.curr_leaf().as_node().slotuse {
            self.curr_slot += 1;
        } else if !self.curr_leaf().next_leaf.is_null() {
            self.curr_leaf = self.curr_leaf().next_leaf;
            self.curr_slot = 1;
        } else {
            self.curr_slot = self.curr_leaf().as_node().slotuse;
        }

        self
    }

    fn equals(&self, other: &Self) -> bool {
        self.curr_leaf == other.curr_leaf && self.curr_slot == other.curr_slot
    }

    fn value(&self) -> &T::ValueType {
        unsafe {
            (*self.curr_leaf).slotdata[self.curr_slot as usize]
                .assume_init_ref()
                .value()
        }
    }
}
