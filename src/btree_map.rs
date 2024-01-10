use crate::btree_base::{
    btree::BTree,
    btree_traits::{BTreeParams, InnerValueType},
    iter::TreeIterator,
    DefaultBTreeConfig,
};

pub struct BTreeMap<T: BTreeParams>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    _tree: BTree<T>,
}

impl<T: BTreeParams> BTreeMap<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub fn is_empty(&self) -> bool {
        self._tree.empty()
    }

    pub fn len(&self) -> usize {
        self._tree.size()
    }

    pub fn contains_key(&self, key: &T::KeyType) -> bool {
        self._tree.exists(key)
    }

    pub fn put(&mut self, key: T::KeyType, value: T::ValueType) {
        if self._tree.exists(&key) {
            self._tree.erase(&key);
        }
        self._tree.insert(InnerValueType(key, value));
    }

    pub fn get(&self, key: &T::KeyType) -> Option<&T::ValueType> {
        let it = self._tree.find(key);
        if it.equals(&self._tree.end()) {
            None
        } else {
            let val = unsafe { (*it.curr_leaf).slotdata[it.curr_slot as usize].assume_init_ref() };

            Some(val.value())
        }
    }

    pub fn remove(&mut self, key: &T::KeyType) {
        self._tree.erase(key);
    }

    pub fn new() -> Self {
        Self {
            _tree: BTree::new(),
        }
    }
}

pub type DefaultBTreeMap<K, V> = BTreeMap<DefaultBTreeConfig<K, V>>;
