use std::fmt::Debug;
use std::{marker::PhantomData, mem::size_of};

// Traits bound
pub trait KeyOfValue<K, V>: Clone + Debug {
    fn get(value: &V) -> &K;
}
pub trait KeyComparator<T>: Clone + Debug {
    fn new() -> Self;
    fn less(&self, lhs: &T, rhs: &T) -> bool;
}

#[derive(Clone, Debug)]
pub struct InnerValueType<T: BTreeParams>(pub T::KeyType, pub T::ValueType);

impl<T: BTreeParams> InnerValueType<T> {
    pub fn key(&self) -> &T::KeyType {
        &self.0
    }

    pub fn value(&self) -> &T::ValueType {
        &self.1
    }
}

impl<T: BTreeParams> KeyOfValue<T::KeyType, InnerValueType<T>> for InnerValueType<T> {
    fn get(value: &InnerValueType<T>) -> &T::KeyType {
        value.key()
    }
}

#[derive(Clone, Debug)]
pub struct ValueCompare<'a, T: BTreeParams> {
    key_comp: &'a T::KeyCompareType,
}

impl<'a, T: BTreeParams> ValueCompare<'a, T> {
    pub fn new(key_comp: &'a T::KeyCompareType) -> Self {
        Self { key_comp }
    }

    pub fn less(&self, x: &InnerValueType<T>, y: &InnerValueType<T>) -> bool {
        self.key_comp.less(&x.0, &y.0)
    }
}

pub trait BTreeTraits: Clone + Debug {
    const LEAF_SLOTS: usize;
    const INNER_SLOTS: usize;
    const BINSEARCH_THRESHOLD: usize;
}

#[derive(Clone, Debug)]
pub struct DefaultBTreeTraits<K: Clone + Debug, V: Clone + Debug> {
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

const fn _max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}

impl<K: Clone + Debug, V: Clone + Debug> BTreeTraits for DefaultBTreeTraits<K, V> {
    const LEAF_SLOTS: usize = _max(8, 256 / size_of::<V>());
    const INNER_SLOTS: usize = _max(8, 256 / (size_of::<K>() + size_of::<*const ()>()));
    const BINSEARCH_THRESHOLD: usize = 256;
}

pub trait BTreeParams: Clone + Debug {
    type KeyType: Clone + Debug + Default;
    type ValueType: Clone + Debug;
    type KeyCompareType: KeyComparator<Self::KeyType>;
    type KeyOfValueType: KeyOfValue<Self::KeyType, InnerValueType<Self>>;
    type Traits: BTreeTraits;
    const LEAF_SLOTMAX: usize;
    const INNER_SLOTMAX: usize;
    const LEAF_SLOTMIN: usize;
    const INNER_SLOTMIN: usize;
    const ALLOW_DUPLICATE: bool;
    const SELF_VERIFY: bool;
}

#[derive(Clone, Debug)]
pub struct _BTree<
    TKey: Clone + Debug + Default,
    TValue: Clone + Debug,
    TCompare,
    Traits: BTreeTraits,
> where
    [(); Traits::LEAF_SLOTS]: Sized,
    [(); Traits::INNER_SLOTS + 1]: Sized,
{
    _phantom_key: PhantomData<TKey>,
    _phantom_value: PhantomData<TValue>,
    _phantom_compare: PhantomData<TCompare>,
    _phantom_traits: PhantomData<Traits>,
}

impl<
        TKey: Clone + Debug + Default,
        TValue: Clone + Debug,
        TCompare: KeyComparator<TKey>,
        TTraits: BTreeTraits,
    > BTreeParams for _BTree<TKey, TValue, TCompare, TTraits>
where
    [(); TTraits::LEAF_SLOTS]: Sized,
    [(); TTraits::INNER_SLOTS + 1]: Sized,
{
    type KeyType = TKey;
    type ValueType = TValue;
    type KeyCompareType = TCompare;
    type KeyOfValueType = InnerValueType<Self>;
    type Traits = TTraits;
    const LEAF_SLOTMAX: usize = TTraits::LEAF_SLOTS;
    const INNER_SLOTMAX: usize = TTraits::INNER_SLOTS;
    const LEAF_SLOTMIN: usize = Self::LEAF_SLOTMAX / 2;
    const INNER_SLOTMIN: usize = Self::INNER_SLOTMAX / 2;
    const ALLOW_DUPLICATE: bool = false;
    const SELF_VERIFY: bool = false;
}

#[cfg(test)]
#[test]
fn test_btree_traits() {
    assert_eq!(DefaultBTreeTraits::<u64, u64>::LEAF_SLOTS, 32);
    assert_eq!(DefaultBTreeTraits::<u64, u64>::INNER_SLOTS, 16);
    assert_eq!(DefaultBTreeTraits::<u64, u64>::BINSEARCH_THRESHOLD, 256);
}
