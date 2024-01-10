pub mod btree;
pub mod btree_traits;
mod deletion;
pub mod iter;
mod macros;
mod node;
mod tree_stats;

use self::{
    btree::BTree,
    btree_traits::{DefaultBTreeTraits, KeyComparator, _BTree},
};
use std::{fmt::Debug, marker::PhantomData};

#[derive(Clone, Debug)]
pub struct DefaultKeyComparator<T> {
    _t: PhantomData<T>,
}
impl<T> KeyComparator<T> for DefaultKeyComparator<T>
where
    T: Ord + Clone + Debug,
{
    fn new() -> Self {
        Self { _t: PhantomData }
    }
    fn less(&self, lhs: &T, rhs: &T) -> bool {
        lhs < rhs
    }
}

pub type DefaultBTreeConfig<K, V> = _BTree<K, V, DefaultKeyComparator<K>, DefaultBTreeTraits<K, V>>;
pub type DefaultBTree<K, V> = BTree<DefaultBTreeConfig<K, V>>;
