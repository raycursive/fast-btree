use std::{
    mem::{size_of_val, MaybeUninit},
    ptr::null_mut,
};

use super::{
    btree_traits::{
        BTreeParams, BTreeTraits, InnerValueType, KeyComparator, KeyOfValue, ValueCompare,
    },
    deletion::{DeletionResult, DeletionResultFlags},
    iter::{BTreeIterator, BTreeReverseIterator},
    macros::memcopy,
    node::{InnerNode, LeafNode, Node, NodeImpl},
    tree_stats::TreeStats,
};

#[repr(C)]
pub struct BTree<T: BTreeParams>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    root_: *mut Node<T>,
    head_leaf_: *mut LeafNode<T>,
    tail_leaf_: *mut LeafNode<T>,
    stats_: TreeStats<T>,
    key_less: T::KeyCompareType,
}

/// Convenient Key Comparison Functions Generated From key_less
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    // key and value comparison functions
    pub fn key_comp(&self) -> &T::KeyCompareType {
        &self.key_less
    }

    pub fn value_comp(&self) -> ValueCompare<T> {
        ValueCompare::new(&self.key_less)
    }
    // convenient key comparison functions generated from key_less
    fn key_less(&self, a: &T::KeyType, b: &T::KeyType) -> bool {
        self.key_less.less(a, b)
    }

    fn key_lessequal(&self, a: &T::KeyType, b: &T::KeyType) -> bool {
        !self.key_less.less(b, a)
    }

    #[allow(dead_code)]
    fn key_greater(&self, a: &T::KeyType, b: &T::KeyType) -> bool {
        self.key_less.less(b, a)
    }

    #[allow(dead_code)]
    fn key_greaterequal(&self, a: &T::KeyType, b: &T::KeyType) -> bool {
        !self.key_less.less(a, b)
    }
    fn key_equal(&self, a: &T::KeyType, b: &T::KeyType) -> bool {
        !self.key_less.less(a, b) && !self.key_less.less(b, a)
    }
}

impl<T: BTreeParams> Drop for BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn drop(&mut self) {
        if !self.root_.is_null() {
            self.clear_recursive(self.root_);
            self.free_node(self.root_);

            self.root_ = null_mut();
            self.head_leaf_ = null_mut();
            self.tail_leaf_ = null_mut();

            self.stats_ = TreeStats::<T>::new();
        }

        debug_assert!(self.stats_.size == 0);
    }
}
/// node object allocation and deallocation functions
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn free_node(&mut self, node: *mut Node<T>) {
        let n = unsafe { Box::from_raw(node) };
        if n.is_leafnode() {
            self.stats_.leaves -= 1;
        } else {
            self.stats_.inner_nodes -= 1;
        }
    }

    fn clear_recursive(&mut self, node: *mut Node<T>) {
        let n = unsafe { &mut *node };
        if n.is_leafnode() {
            let leaf = unsafe { &mut *node }.as_leaf_mut();
            for _ in 0..leaf.slotuse {
                // deleted by destructor
            }
        } else {
            let inner = unsafe { &mut *node }.as_inner_mut();
            for i in 0..=inner.slotuse {
                self.clear_recursive(inner.get_child(i as usize));
                self.free_node(inner.get_child(i as usize));
            }
        }
    }

    fn new_inner(&mut self, level: u16) -> *mut Node<T> {
        let new_node = Node::new_inner(level);
        self.stats_.inner_nodes += 1;

        new_node
    }

    pub fn new_leaf(&mut self) -> *mut Node<T> {
        let new_node = Node::new_leaf();
        self.stats_.leaves += 1;

        new_node
    }
}

/// B+ Tree Node Binary Search Functions
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn find_lower(&self, n: &impl NodeImpl<T>, key: &T::KeyType) -> u16 {
        if size_of_val(n) > T::Traits::BINSEARCH_THRESHOLD {
            let _slotuse = n.slotuse();
            if _slotuse == 0 {
                return 0;
            }
            let mut lo: u16 = 0;
            let mut hi: u16 = _slotuse;
            while lo < hi {
                let mid = (lo + hi) >> 1;
                if self.key_lessequal(key, n.key(mid as usize)) {
                    hi = mid
                } else {
                    lo = mid + 1
                }
            }
            return lo;
        } else {
            // for nodes <= binsearch_threshold do linear search
            let mut lo: u16 = 0;
            while lo < n.slotuse() && self.key_less(n.key(lo as usize), key) {
                lo += 1;
            }
            return lo;
        }
    }

    fn find_upper(&self, n: &impl NodeImpl<T>, key: &T::KeyType) -> u16 {
        if size_of_val(n) > T::Traits::BINSEARCH_THRESHOLD {
            let _slotuse = n.slotuse();
            if _slotuse == 0 {
                return 0;
            }
            let mut lo: u16 = 0;
            let mut hi: u16 = _slotuse;
            while lo < hi {
                let mid = (lo + hi) >> 1;
                if self.key_less(key, n.key(mid as usize)) {
                    hi = mid
                } else {
                    lo = mid + 1
                }
            }
            return lo;
        } else {
            // for nodes <= binsearch_threshold do linear search
            let mut lo: u16 = 0;
            while lo < n.slotuse() && self.key_lessequal(n.key(lo as usize), key) {
                lo += 1;
            }
            return lo;
        }
    }
}

/// Access Functions to the item count
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    // access functions to the item count
    pub fn size(&self) -> usize {
        self.stats_.size
    }

    pub fn empty(&self) -> bool {
        self.size() == 0 as usize
    }

    pub fn max_size(&self) -> usize {
        usize::MAX
    }

    pub fn get_stats(&self) -> &TreeStats<T> {
        &self.stats_
    }
}

/// Access function querying the tree by descending to a leaf
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    /// Non-STL function checking whether a key is in the B+ tree. The same as
    /// (find(k) != end()) or (count() != 0).
    pub fn exists(&self, key: &T::KeyType) -> bool {
        let mut n = self.root_;
        if n.is_null() {
            return false;
        }

        while !unsafe { (&*n).is_leafnode() } {
            let inner = unsafe { &*n }.as_inner();
            let slot = self.find_lower(inner, key);
            n = inner.get_child(slot as usize);
        }

        let leaf = unsafe { &*n }.as_leaf();
        let slot = self.find_lower(leaf, key);

        slot < leaf.slotuse && self.key_equal(key, leaf.key(slot as usize))
    }

    pub fn begin(&self) -> BTreeIterator<T> {
        BTreeIterator::new(self.head_leaf_, 0)
    }

    pub fn end(&self) -> BTreeIterator<T> {
        BTreeIterator::new(
            self.tail_leaf_,
            if self.tail_leaf_.is_null() {
                0
            } else {
                unsafe { &*self.tail_leaf_ }.slotuse
            },
        )
    }

    pub fn rbegin(&self) -> BTreeReverseIterator<T> {
        BTreeReverseIterator::new(
            self.tail_leaf_,
            if self.tail_leaf_.is_null() {
                0
            } else {
                unsafe { &*self.tail_leaf_ }.slotuse
            },
        )
    }

    pub fn rend(&self) -> BTreeReverseIterator<T> {
        BTreeReverseIterator::new(self.head_leaf_, 0)
    }

    /// Tries to locate a key in the B+ tree and returns an constant iterator to
    /// the key/data slot if found. If unsuccessful it returns end().
    pub fn find(&self, key: &T::KeyType) -> BTreeIterator<T> {
        let mut n = self.root_;
        if n.is_null() {
            return self.end();
        }

        while !unsafe { (&*n).is_leafnode() } {
            let inner = unsafe { &*n }.as_inner();
            let slot = self.find_lower(inner, key);
            n = unsafe { &*n }.as_inner().get_child(slot as usize);
        }

        let leaf = unsafe { &*n }.as_leaf();
        let slot = self.find_lower(leaf, key);

        if slot < leaf.slotuse && self.key_equal(key, leaf.key(slot as usize)) {
            BTreeIterator::new(leaf as *const LeafNode<T> as *mut LeafNode<T>, slot)
        } else {
            self.end()
        }
    }

    /// Tries to locate a key in the B+ tree and returns the number of identical
    /// key entries found.
    pub fn count(&self, key: &T::KeyType) -> usize {
        let mut n = self.root_;
        if n.is_null() {
            return 0;
        }

        while !unsafe { (&*n).is_leafnode() } {
            let inner = unsafe { &*n }.as_inner();
            let slot = self.find_lower(inner, key);
            n = unsafe { &*n }.as_inner().get_child(slot as usize);
        }

        let mut leaf = unsafe { &*n }.as_leaf();
        let mut slot = self.find_lower(leaf, key);
        let mut num: usize = 0;
        while slot < leaf.slotuse && self.key_equal(key, leaf.key(slot as usize)) {
            num += 1;
            slot += 1;
            if slot > leaf.slotuse {
                let _leaf = leaf.next_leaf;
                if _leaf.is_null() {
                    break;
                }
                leaf = unsafe { &*_leaf };
                slot = 0;
            }
        }

        num
    }

    /// Searches the B+ tree and returns an iterator to the first pair equal to
    /// or greater than key, or end() if all keys are smaller.
    pub fn lower_bound(&self, key: &T::KeyType) -> BTreeIterator<T> {
        let mut n = self.root_;
        if n.is_null() {
            return self.end();
        }

        while !unsafe { (&*n).is_leafnode() } {
            let inner = unsafe { &*n }.as_inner();
            let slot = self.find_lower(inner, key);
            n = unsafe { &*n }.as_inner().get_child(slot as usize);
        }

        let leaf = unsafe { &*n }.as_leaf();
        let slot = self.find_lower(leaf, key);

        BTreeIterator::new(leaf as *const LeafNode<T> as *mut LeafNode<T>, slot)
    }

    /// Searches the B+ tree and returns an iterator to the first pair greater
    /// than key, or end() if all keys are smaller or equal.
    pub fn upper_bound(&self, key: &T::KeyType) -> BTreeIterator<T> {
        let mut n = self.root_;
        if n.is_null() {
            return self.end();
        }

        while !unsafe { (&*n).is_leafnode() } {
            let inner = unsafe { &*n }.as_inner();
            let slot = self.find_upper(inner, key);
            n = unsafe { &*n }.as_inner().get_child(slot as usize);
        }

        let leaf = unsafe { &*n }.as_leaf();
        let slot = self.find_upper(leaf, key);

        BTreeIterator::new(leaf as *const LeafNode<T> as *mut LeafNode<T>, slot)
    }

    /// Searches the B+ tree and returns both lower_bound() and upper_bound().
    pub fn equal_range(&self, key: &T::KeyType) -> (BTreeIterator<T>, BTreeIterator<T>) {
        (self.lower_bound(key), self.upper_bound(key))
    }
}

/// Insertion
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    /// Attempt to insert a key/data pair into the B+ tree. If the tree does not
    /// allow duplicate keys, then the insert may fail if it is already present.
    pub fn insert(&mut self, x: InnerValueType<T>) -> (BTreeIterator<T>, bool) {
        self.insert_start(T::KeyOfValueType::get(&x), &x)
    }

    /// Start the insertion descent at the current root and handle root splits.
    /// Returns true if the item was inserted
    fn insert_start(
        &mut self,
        key: &T::KeyType,
        value: &InnerValueType<T>,
    ) -> (BTreeIterator<T>, bool) {
        let mut newchild: *mut Node<T> = null_mut();
        let mut newkey: T::KeyType = T::KeyType::default();
        if self.root_.is_null() {
            let new_node = self.new_leaf();
            let new_leaf = unsafe { &mut *new_node }.as_leaf_mut();
            let _new_leaf = new_leaf as *mut LeafNode<T>;
            self.root_ = new_node;
            self.head_leaf_ = _new_leaf;
            self.tail_leaf_ = _new_leaf;
        }

        let r = self.insert_descend(self.root_, key, value, &mut newkey, &mut newchild);

        if !newchild.is_null() {
            let new_node = self.new_inner(unsafe { &*self.root_ }.level + 1);
            let new_inner = unsafe { &mut *new_node }.as_inner_mut();
            new_inner.slotkey[0] = MaybeUninit::new(newkey);
            new_inner.childid[0] = MaybeUninit::new(self.root_);
            new_inner.childid[1] = MaybeUninit::new(newchild);
            new_inner.slotuse = 1;
            self.root_ = new_node;
        }

        if r.1 {
            self.stats_.size += 1;
        }

        r
    }

    /// Insert an item into the B+ tree.
    ///
    /// Descend down the nodes to a leaf, insert the key/data pair in a free
    /// slot. If the node overflows, then it must be split and the new split node
    /// inserted into the parent. Unroll / this splitting up to the root.
    fn insert_descend(
        &mut self,
        n: *mut Node<T>,
        key: &T::KeyType,
        value: &InnerValueType<T>,
        splitkey: &mut T::KeyType,
        splitnode: &mut *mut Node<T>,
    ) -> (BTreeIterator<T>, bool) {
        let n_ref = unsafe { &*n };
        let n_mut = unsafe { &mut *n };
        if !n_ref.is_leafnode() {
            let mut inner = n_mut.as_inner_mut();
            let mut newkey = T::KeyType::default();
            let mut newchild: *mut Node<T> = null_mut();

            let mut slot = self.find_lower(inner, key);
            let r = self.insert_descend(
                inner.get_child(slot as usize),
                key,
                value,
                &mut newkey,
                &mut newchild,
            );

            log::debug!(
                "BTree::insert_descend into {:p}",
                inner.get_child(slot as usize)
            );

            if !newchild.is_null() {
                log::debug!(
                    "BTree::insert_descend newchild with key {:?} node {:p} at slot {}",
                    newkey,
                    newchild,
                    slot
                );

                if inner.is_full() {
                    self.split_inner_node(inner, splitkey, splitnode, slot as u32);
                    log::debug!("BTree::insert_descend done split_inner: putslot: {}, putkey: {:?}, upkey: {:?}", slot, newkey, *splitkey);

                    // check if insert slot is in the split sibling node
                    log::debug!(
                        "BTREE::insert_descend switch: {} > {}",
                        slot,
                        inner.slotuse + 1
                    );

                    if slot == inner.slotuse + 1 && inner.slotuse < unsafe { (**splitnode).slotuse }
                    {
                        // special case when the insert slot matches the split
                        // place between the two nodes, then the insert key
                        // becomes the split key.
                        debug_assert!(inner.slotuse as usize + 1 < T::INNER_SLOTMAX);
                        let split = unsafe { &mut **splitnode }.as_inner_mut();

                        // move the split key and it's datum into the left node
                        inner.slotkey[inner.slotuse as usize] =
                            MaybeUninit::new((*splitkey).clone());
                        inner.childid[inner.slotuse as usize + 1] = split.childid[0];
                        inner.slotuse += 1;

                        // set new split key and move corresponding datum into
                        // right node
                        split.childid[0] = MaybeUninit::new(newchild);
                        *splitkey = newkey;

                        return r;
                    } else if slot >= inner.slotuse + 1 {
                        // in case the insert slot is in the newly create split
                        // node, we reuse the code below.

                        slot -= inner.slotuse + 1;
                        inner = unsafe { &mut **splitnode }.as_inner_mut();
                        log::debug!(
                            "BTree::insert_descend switching to splitted node {:p} at slot {}",
                            inner,
                            slot
                        );
                    }
                }

                // move items and put pointer to child node into correct slot
                debug_assert!(slot <= inner.slotuse);

                memcopy!(inner, inner, slotkey, slot, inner.slotuse, slot + 1);
                memcopy!(inner, inner, childid, slot, inner.slotuse + 1, slot + 1);
                inner.slotkey[slot as usize] = MaybeUninit::new(newkey);
                inner.childid[slot as usize + 1] = MaybeUninit::new(newchild);
                inner.slotuse += 1;
            }

            r
        } else {
            let mut leaf = n_mut.as_leaf_mut();
            let mut slot = self.find_lower(leaf, key);
            if !T::ALLOW_DUPLICATE
                && slot < leaf.slotuse
                && self.key_equal(key, leaf.key(slot as usize))
            {
                return (BTreeIterator::new(leaf as *mut LeafNode<T>, slot), false);
            }

            if leaf.is_full() {
                self.split_leaf_node(leaf, splitkey, splitnode);

                // check if insert slot is in the split sibling node
                if slot >= leaf.slotuse {
                    slot -= leaf.slotuse;
                    leaf = unsafe { &mut **splitnode }.as_leaf_mut();
                }
            }

            debug_assert!(slot <= leaf.slotuse);
            memcopy!(leaf, leaf, slotdata, slot, leaf.slotuse, slot + 1);
            leaf.slotdata[slot as usize] = MaybeUninit::new(value.clone());
            leaf.slotuse += 1;

            if !splitnode.is_null()
                && (leaf as *mut _) != (unsafe { &mut **splitnode }.as_leaf_mut() as *mut _)
                && slot == leaf.slotuse - 1
            {
                *splitkey = key.clone();
            }

            (BTreeIterator::new(leaf as *mut LeafNode<T>, slot), true)
        }
    }

    /// Split up a leaf node into two equally-filled sibling leaves. Returns the
    /// new nodes and it's insertion key in the two parameters.
    pub fn split_leaf_node(
        &mut self,
        leaf: &mut LeafNode<T>,
        out_newkey: &mut T::KeyType,
        out_newleaf: &mut *mut Node<T>,
    ) {
        debug_assert!(leaf.is_full());

        let mid: u16 = leaf.slotuse >> 1;

        log::debug!("BTree::split_leaf_node on {:p}", leaf);

        let _newleaf = self.new_leaf();
        let newleaf = unsafe { &mut *_newleaf }.as_leaf_mut();

        newleaf.slotuse = leaf.slotuse - mid;

        newleaf.next_leaf = leaf.next_leaf;

        if newleaf.next_leaf.is_null() {
            debug_assert!(leaf as *mut LeafNode<T> == self.tail_leaf_);
            self.tail_leaf_ = newleaf as *mut LeafNode<T>;
        } else {
            unsafe { &mut *(newleaf.next_leaf) }.prev_leaf = newleaf as *mut LeafNode<T>;
        }

        memcopy!(leaf, newleaf, slotdata, mid, leaf.slotuse, 0);
        leaf.slotuse = mid;
        leaf.next_leaf = newleaf as *mut LeafNode<T>;
        newleaf.prev_leaf = leaf as *mut LeafNode<T>;

        *out_newkey = leaf.key(leaf.slotuse as usize - 1).clone();
        *out_newleaf = _newleaf;
    }

    /// Split up an inner node into two equally-filled sibling nodes. Returns
    /// the new nodes and it's insertion key in the two parameters. Requires the
    /// slot of the item will be inserted, so the nodes will be the same size
    /// after the insert.
    pub fn split_inner_node(
        &mut self,
        inner: &mut InnerNode<T>,
        out_newkey: &mut T::KeyType,
        out_newinner: &mut *mut Node<T>,
        addslot: u32,
    ) {
        debug_assert!(inner.is_full());

        let mut mid: u16 = inner.slotuse >> 1;

        log::debug!("BTree::split_inner: mid {:p} addslot {}", inner, addslot);

        // if the split is uneven and the overflowing item will be put into the
        // larger node, then the smaller split node may underflow
        if addslot <= mid as u32 && mid > inner.slotuse - (mid + 1) {
            mid -= 1;
        }

        log::debug!("BTree::split_inner: mid {:p} addslot {}", inner, addslot);

        log::debug!(
            "BTree::split_inner_node on {:p} into two nodes {} and {} sized",
            inner,
            mid,
            inner.slotuse - (mid + 1)
        );

        let _newinner = self.new_inner(inner.level);
        let newinner = unsafe { &mut *_newinner }.as_inner_mut();

        newinner.slotuse = inner.slotuse - (mid + 1);

        memcopy!(inner, newinner, slotkey, mid + 1, inner.slotuse, 0);
        memcopy!(inner, newinner, childid, mid + 1, inner.slotuse + 1, 0);
        inner.slotuse = mid;

        *out_newkey = inner.key(mid as usize).clone();
        *out_newinner = _newinner;
    }
}

/// Erase
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub fn erase_one(&mut self, key: &T::KeyType) -> bool {
        log::debug!("Btree::erase_one({:?}) on btree size {}", key, self.size());
        if self.root_.is_null() {
            return false;
        }

        let result = self.erase_one_descend(
            key,
            self.root_,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
        );
        if !result.has(DeletionResultFlags::NotFound) {
            self.stats_.size -= 1;
        }

        return !result.has(DeletionResultFlags::NotFound);
    }

    pub fn erase(&mut self, key: &T::KeyType) -> usize {
        let mut c = 0;
        while self.erase_one(key) {
            c += 1;
            if !T::ALLOW_DUPLICATE {
                break;
            }
        }
        return c;
    }

    fn erase_one_descend(
        &mut self,
        key: &T::KeyType,
        curr: *mut Node<T>,
        left: *mut Node<T>,
        right: *mut Node<T>,
        left_parent: *mut InnerNode<T>,
        right_parent: *mut InnerNode<T>,
        parent: *mut InnerNode<T>,
        parentslot: u16,
    ) -> DeletionResult<T> {
        let curr_ref = unsafe { &*curr };
        let curr_mut = unsafe { &mut *curr };
        if curr_ref.is_leafnode() {
            let leaf = curr_mut.as_leaf_mut();
            let slot = self.find_lower(leaf, key);

            if slot >= leaf.slotuse || !self.key_equal(key, leaf.key(slot as usize)) {
                log::debug!("Could not find key {:?} to erase.", key);
                return DeletionResult::new(DeletionResultFlags::NotFound);
            }

            log::debug!("Found key in leaf {:p} at slot {}", curr, slot);

            memcopy!(leaf, leaf, slotdata, slot + 1, leaf.slotuse, slot);
            leaf.slotuse -= 1;

            let mut myres = DeletionResult::<T>::new(DeletionResultFlags::Ok);

            // if the last key of the leaf was changed, the parent is notified
            // and updates the key of this leaf
            if slot == leaf.slotuse {
                if !parent.is_null() && parentslot < unsafe { &*parent }.slotuse {
                    debug_assert!(unsafe { &*parent }.get_child(parentslot as usize) == curr);
                    unsafe { &mut *parent }.slotkey[parentslot as usize] =
                        MaybeUninit::new(leaf.key(leaf.slotuse as usize - 1).clone());
                } else {
                    if leaf.slotuse >= 1 {
                        log::debug!(
                            "Scheduling lastkeyupdate: key {:?}",
                            leaf.key(leaf.slotuse as usize - 1)
                        );
                        myres |= DeletionResult::new_with_key(
                            DeletionResultFlags::UpdateLastKey,
                            leaf.key(leaf.slotuse as usize - 1).clone(),
                        );
                    } else {
                        debug_assert!(curr == self.root_);
                    }
                }
            }

            if leaf.is_underflow() && !(curr == self.root_ && leaf.slotuse >= 1) {
                // determine what to do about the underflow

                // case : if this empty leaf is the root, then delete all nodes
                // and set root to nullptr.
                if left.is_null() && right.is_null() {
                    debug_assert!(curr == self.root_);
                    debug_assert!(leaf.slotuse == 0);

                    self.free_node(self.root_);

                    self.root_ = null_mut();
                    self.head_leaf_ = null_mut();
                    self.tail_leaf_ = null_mut();
                    return DeletionResult::new(DeletionResultFlags::Ok);
                }
                // case : if both left and right leaves would underflow in case
                // of a shift, then merging is necessary. choose the more local
                // merger with our parent
                else if (left.is_null() || unsafe { &*left }.as_leaf().is_few())
                    && (right.is_null() || unsafe { &*right }.as_leaf().is_few())
                {
                    if left_parent == parent {
                        myres |=
                            self.merge_leaves(unsafe { &mut *left }.as_leaf_mut(), leaf, unsafe {
                                &mut *left_parent
                            });
                    } else {
                        myres |=
                            self.merge_leaves(leaf, unsafe { &mut *right }.as_leaf_mut(), unsafe {
                                &mut *right_parent
                            });
                    }
                }
                // case : the right leaf has extra data, so balance right with
                // current
                else if !left.is_null()
                    && unsafe { &*left }.as_leaf().is_few()
                    && !right.is_null()
                    && !unsafe { &*right }.as_leaf().is_few()
                {
                    if right_parent == parent {
                        myres |= Self::shift_left_leaf(
                            leaf,
                            unsafe { &mut *right }.as_leaf_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    } else {
                        myres |=
                            self.merge_leaves(unsafe { &mut *left }.as_leaf_mut(), leaf, unsafe {
                                &mut *left_parent
                            });
                    }
                }
                // case : the left leaf has extra data, so balance left with
                // current
                else if !left.is_null()
                    && !unsafe { &*left }.as_leaf().is_few()
                    && !right.is_null()
                    && unsafe { &*right }.as_leaf().is_few()
                {
                    if left_parent == parent {
                        Self::shift_right_leaf(
                            unsafe { &mut *left }.as_leaf_mut(),
                            leaf,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    } else {
                        myres |=
                            self.merge_leaves(leaf, unsafe { &mut *right }.as_leaf_mut(), unsafe {
                                &mut *right_parent
                            });
                    }
                }
                // case : both the leaf and right leaves have extra data and our
                // parent, choose the leaf with more data
                else if left_parent == right_parent {
                    if unsafe { &*left }.slotuse <= unsafe { &*right }.slotuse {
                        myres |= Self::shift_left_leaf(
                            leaf,
                            unsafe { &mut *right }.as_leaf_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    } else {
                        Self::shift_right_leaf(
                            unsafe { &mut *left }.as_leaf_mut(),
                            leaf,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    }
                } else {
                    if left_parent == parent {
                        Self::shift_right_leaf(
                            unsafe { &mut *left }.as_leaf_mut(),
                            leaf,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    } else {
                        myres |= Self::shift_left_leaf(
                            leaf,
                            unsafe { &mut *right }.as_leaf_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    }
                }
            }

            return myres;
        } else {
            let inner = curr_mut.as_inner_mut();
            let mut slot = self.find_lower(inner, key);
            let myleft: *mut Node<T>;
            let myright: *mut Node<T>;
            let myleft_parent: *mut InnerNode<T>;
            let myright_parent: *mut InnerNode<T>;

            if slot == 0 {
                myleft = if left.is_null() {
                    null_mut()
                } else {
                    let left_ref = unsafe { &*left }.as_inner();
                    left_ref.get_child(left_ref.slotuse as usize - 1)
                };
                myleft_parent = left_parent;
            } else {
                myleft = inner.get_child(slot as usize - 1);
                myleft_parent = inner;
            }

            if slot == inner.slotuse {
                myright = if right.is_null() {
                    null_mut()
                } else {
                    unsafe { &*right }.as_inner().get_child(0)
                };
                myright_parent = right_parent;
            } else {
                myright = inner.get_child(slot as usize + 1);
                myright_parent = inner;
            }

            log::debug!(
                "erase_one_descend into {:p}",
                inner.get_child(slot as usize)
            );
            let result = self.erase_one_descend(
                key,
                inner.get_child(slot as usize),
                myleft,
                myright,
                myleft_parent,
                myright_parent,
                inner,
                slot,
            );

            let mut myres = DeletionResult::<T>::new(DeletionResultFlags::Ok);

            if result.has(DeletionResultFlags::NotFound) {
                return result;
            }

            if result.has(DeletionResultFlags::UpdateLastKey) {
                if !parent.is_null() && parentslot < unsafe { &*parent }.slotuse {
                    log::debug!(
                        "Fixing lastkeyupdate: key {:?} into parent {:p} at parentslot {}",
                        result.last_key.as_ref(),
                        parent,
                        parentslot,
                    );
                    debug_assert!(unsafe { &*parent }.get_child(parentslot as usize) == curr);
                    unsafe { &mut *parent }.slotkey[parentslot as usize] = MaybeUninit::new(
                        result
                            .last_key
                            .as_ref()
                            .expect("key shouldn't be null in UpdateLastKey")
                            .clone(),
                    );
                } else {
                    log::debug!(
                        "Forwarding lastkeyupdate: key {:?}",
                        result.last_key.as_ref()
                    );
                    myres |= DeletionResult::new_with_key(
                        DeletionResultFlags::UpdateLastKey,
                        result
                            .last_key
                            .as_ref()
                            .expect("key shouldn't be null in UpdateLastKey")
                            .clone(),
                    );
                }
            }

            if result.has(DeletionResultFlags::FixMerge) {
                // either the current node or the next is empty and should be
                // removed
                if unsafe { &*inner.get_child(slot as usize) }.slotuse != 0 {
                    slot += 1;
                }

                // this is the child slot invalidated by the merge
                debug_assert!(unsafe { &*inner.get_child(slot as usize) }.slotuse == 0);

                self.free_node(inner.get_child(slot as usize));

                memcopy!(inner, inner, slotkey, slot, inner.slotuse, slot - 1);
                memcopy!(inner, inner, childid, slot + 1, inner.slotuse + 1, slot);
                inner.slotuse -= 1;

                if inner.level == 1 {
                    // fix split key for children leaves
                    slot -= 1;
                    let child = unsafe { &*inner.get_child(slot as usize) }.as_leaf();
                    inner.slotkey[slot as usize] =
                        MaybeUninit::new(child.key(child.slotuse as usize - 1).clone());
                }
            }

            if inner.is_underflow() && !(curr == self.root_ && inner.slotuse >= 1) {
                // case: the inner node is the root and has just one child. that
                // child becomes the new root
                if left.is_null() && right.is_null() {
                    debug_assert!(curr == self.root_);
                    debug_assert!(inner.slotuse == 0);

                    self.root_ = inner.get_child(0);

                    inner.slotuse = 0;
                    self.free_node(curr);

                    return DeletionResult::new(DeletionResultFlags::Ok);
                }
                // case: if both left and right leaves would underflow in case
                // of a shift, then merging is necessary. choose the more local
                // merger with our parent
                else if (left.is_null() || unsafe { &*left }.as_inner().is_few())
                    && (right.is_null() || unsafe { &*right }.as_inner().is_few())
                {
                    if left_parent == parent {
                        myres |= Self::merge_inner(
                            unsafe { &mut *left }.as_inner_mut(),
                            inner,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    } else {
                        myres |= Self::merge_inner(
                            inner,
                            unsafe { &mut *right }.as_inner_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    }
                }
                // case : the right leaf has extra data, so balance right with
                // current
                else if !left.is_null()
                    && unsafe { &*left }.as_inner().is_few()
                    && !right.is_null()
                    && !unsafe { &*right }.as_inner().is_few()
                {
                    if right_parent == parent {
                        Self::shift_left_inner(
                            inner,
                            unsafe { &mut *right }.as_inner_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    } else {
                        myres |= Self::merge_inner(
                            unsafe { &mut *left }.as_inner_mut(),
                            inner,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    }
                }
                // case : the left leaf has extra data, so balance left with
                // current
                else if !left.is_null()
                    && !unsafe { &*left }.as_inner().is_few()
                    && !right.is_null()
                    && unsafe { &*right }.as_inner().is_few()
                {
                    if left_parent == parent {
                        Self::shift_right_inner(
                            unsafe { &mut *left }.as_inner_mut(),
                            inner,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    } else {
                        myres |= Self::merge_inner(
                            inner,
                            unsafe { &mut *right }.as_inner_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    }
                }
                // case : both the leaf and right leaves have extra data and our
                // parent, choose the leaf with more data
                else if left_parent == right_parent {
                    if unsafe { &*left }.slotuse <= unsafe { &*right }.slotuse {
                        Self::shift_left_inner(
                            inner,
                            unsafe { &mut *right }.as_inner_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    } else {
                        Self::shift_right_inner(
                            unsafe { &mut *left }.as_inner_mut(),
                            inner,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    }
                } else {
                    if left_parent == parent {
                        Self::shift_right_inner(
                            unsafe { &mut *left }.as_inner_mut(),
                            inner,
                            unsafe { &mut *left_parent },
                            parentslot - 1,
                        );
                    } else {
                        Self::shift_left_inner(
                            inner,
                            unsafe { &mut *right }.as_inner_mut(),
                            unsafe { &mut *right_parent },
                            parentslot,
                        );
                    }
                }
            }

            return myres;
        }
    }

    /// Merge two leaf nodes. The function moves all key/data pairs from right
    /// to left and sets right's slotuse to zero. The right slot is then removed
    /// by the calling parent node.
    fn merge_leaves(
        &mut self,
        left: &mut LeafNode<T>,
        right: &mut LeafNode<T>,
        parent: &mut InnerNode<T>,
    ) -> DeletionResult<T> {
        log::debug!(
            "Merge leaf nodes {:p} and {:p} with common parent {:p}",
            left,
            right,
            parent
        );
        debug_assert!(parent.level == 1);
        debug_assert!((left.slotuse as usize + right.slotuse as usize) < T::LEAF_SLOTMAX);

        memcopy!(right, left, slotdata, 0, right.slotuse, left.slotuse);

        left.slotuse += right.slotuse;

        left.next_leaf = right.next_leaf;

        if !left.next_leaf.is_null() {
            unsafe { &mut *left.next_leaf }.prev_leaf = left as *mut LeafNode<T>;
        } else {
            self.tail_leaf_ = left;
        }

        right.slotuse = 0;

        DeletionResult::<T>::new(DeletionResultFlags::FixMerge)
    }

    fn merge_inner(
        left: &mut InnerNode<T>,
        right: &mut InnerNode<T>,
        parent: &mut InnerNode<T>,
        parentslot: u16,
    ) -> DeletionResult<T> {
        log::debug!(
            "Merge inner nodes {:p} and {:p} with common parent {:p}.",
            left,
            right,
            parent
        );

        debug_assert!(left.level == right.level);
        debug_assert!(parent.level == left.level + 1);
        debug_assert!(
            unsafe { &*parent.get_child(parentslot as usize) }.as_inner() as *const InnerNode<T>
                as *mut InnerNode<T>
                == left
        );
        debug_assert!((left.slotuse as usize + right.slotuse as usize) < T::INNER_SLOTMAX);

        // retrieve the decision key from parent
        left.slotkey[left.slotuse as usize] =
            MaybeUninit::new(parent.key(parentslot as usize).clone());
        left.slotuse += 1;

        // copy over keys and children from right
        memcopy!(right, left, slotkey, 0, right.slotuse, left.slotuse);
        memcopy!(right, left, childid, 0, right.slotuse + 1, left.slotuse);

        left.slotuse += right.slotuse;
        right.slotuse = 0;

        DeletionResult::<T>::new(DeletionResultFlags::FixMerge)
    }

    fn shift_left_leaf(
        left: &mut LeafNode<T>,
        right: &mut LeafNode<T>,
        parent: &mut InnerNode<T>,
        parentslot: u16,
    ) -> DeletionResult<T> {
        debug_assert!(parent.level == 1);
        debug_assert!(left.next_leaf == right as *mut LeafNode<T>);
        debug_assert!(left as *mut LeafNode<T> == right.prev_leaf);
        debug_assert!(left.slotuse < right.slotuse);
        debug_assert!(
            unsafe { &*parent.get_child(parentslot as usize) }.as_leaf() as *const LeafNode<T>
                as *mut LeafNode<T>
                == left as *mut LeafNode<T>
        );

        let shiftnum = (right.slotuse - left.slotuse) >> 1;

        log::debug!(
            "Shifting(leaf) {} entries to left {:p} from right {:p} with common parent {:p}.",
            shiftnum,
            left,
            right,
            parent,
        );

        debug_assert!((left.slotuse as usize + shiftnum as usize) < T::LEAF_SLOTMAX);

        // copy the first items from the right node to the last slot in the left
        // node.
        memcopy!(right, left, slotdata, 0, shiftnum, left.slotuse);

        left.slotuse += shiftnum;

        // shift all slots in the right node to the left
        memcopy!(right, right, slotdata, shiftnum, right.slotuse, 0);

        right.slotuse -= shiftnum;

        // fixup parent
        if parentslot < parent.slotuse {
            parent.slotkey[parentslot as usize] =
                MaybeUninit::new(left.key(left.slotuse as usize - 1).clone());
            DeletionResult::<T>::new(DeletionResultFlags::Ok)
        } else {
            DeletionResult::<T>::new_with_key(
                DeletionResultFlags::UpdateLastKey,
                left.key(left.slotuse as usize - 1).clone(),
            )
        }
    }

    fn shift_right_leaf(
        left: &mut LeafNode<T>,
        right: &mut LeafNode<T>,
        parent: &mut InnerNode<T>,
        parentslot: u16,
    ) {
        debug_assert!(parent.level == 1);
        debug_assert!(left.next_leaf == right as *mut LeafNode<T>);
        debug_assert!(left as *mut LeafNode<T> == right.prev_leaf);
        debug_assert!(
            unsafe { &*parent.get_child(parentslot as usize) }.as_leaf() as *const LeafNode<T>
                as *mut LeafNode<T>
                == left as *mut LeafNode<T>
        );
        debug_assert!(left.slotuse > right.slotuse);

        let shiftnum = (left.slotuse - right.slotuse) >> 1;

        log::debug!(
            "Shifting(leaf) {} entries to right {:p} from left {:p} with common parent {:p}.",
            shiftnum,
            right,
            left,
            parent,
        );

        if T::SELF_VERIFY {
            let mut leftslot: u16 = 0;
            while leftslot <= parent.slotuse
                && unsafe { &*parent.get_child(leftslot as usize) }.as_leaf() as *const LeafNode<T>
                    as *mut LeafNode<T>
                    != (left as *mut LeafNode<T>)
            {
                leftslot += 1;
            }

            debug_assert!(leftslot < parent.slotuse);
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize) }.as_leaf() as *const LeafNode<T>
                    as *mut LeafNode<T>
                    == left as *mut LeafNode<T>,
            );
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize + 1) }.as_leaf() as *const LeafNode<T>
                    as *mut LeafNode<T>
                    == right as *mut LeafNode<T>,
            );
            debug_assert!(leftslot == parentslot);
        }

        // shift all slots in the right node
        debug_assert!((right.slotuse as usize + shiftnum as usize) < T::LEAF_SLOTMAX);

        memcopy!(right, right, slotdata, 0, right.slotuse, shiftnum);

        right.slotuse += shiftnum;

        // copy the last items from the left node to the first slot in the right
        // node.
        memcopy!(
            left,
            right,
            slotdata,
            left.slotuse - shiftnum,
            left.slotuse,
            0
        );

        left.slotuse -= shiftnum;
        parent.slotkey[parentslot as usize] =
            MaybeUninit::new(left.key(left.slotuse as usize - 1).clone());
    }

    fn shift_left_inner(
        left: &mut InnerNode<T>,
        right: &mut InnerNode<T>,
        parent: &mut InnerNode<T>,
        parentslot: u16,
    ) {
        debug_assert!(left.level == right.level);
        debug_assert!(parent.level == left.level + 1);
        debug_assert!(left.slotuse < right.slotuse);
        debug_assert!(
            unsafe { &*parent.get_child(parentslot as usize) }.as_inner() as *const InnerNode<T>
                as *mut InnerNode<T>
                == left as *mut InnerNode<T>
        );

        let shiftnum = (right.slotuse - left.slotuse) >> 1;

        log::debug!(
            "Shifting(inner) {} entries to left {:p} from right {:p} with common parent {:p}.",
            shiftnum,
            left,
            right,
            parent,
        );

        debug_assert!((left.slotuse as usize + shiftnum as usize) < T::INNER_SLOTMAX);

        if T::SELF_VERIFY {
            let mut leftslot: u16 = 0;
            while leftslot <= parent.slotuse
                && unsafe { &*parent.get_child(leftslot as usize) }.as_inner()
                    as *const InnerNode<T> as *mut InnerNode<T>
                    != (left as *mut InnerNode<T>)
            {
                leftslot += 1;
            }

            debug_assert!(leftslot < parent.slotuse);
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize) }.as_inner() as *const InnerNode<T>
                    as *mut InnerNode<T>
                    == left as *mut InnerNode<T>,
            );
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize + 1) }.as_inner()
                    as *const InnerNode<T> as *mut InnerNode<T>
                    == right as *mut InnerNode<T>,
            );
            debug_assert!(leftslot == parentslot);
        }

        // copy the parent's decision slotkey and childid to the first new key
        // on the left
        left.slotkey[left.slotuse as usize] =
            MaybeUninit::new(parent.key(parentslot as usize).clone());
        left.slotuse += 1;

        // copy the other items from the right node to the last slots in the
        // left node.
        memcopy!(right, left, slotkey, 0, shiftnum - 1, left.slotuse);
        memcopy!(right, left, childid, 0, shiftnum, left.slotuse);

        left.slotuse += shiftnum - 1;

        // fixup parent
        parent.slotkey[parentslot as usize] =
            MaybeUninit::new(right.key(shiftnum as usize - 1).clone());

        // shift all slots in the right node
        memcopy!(right, right, slotkey, shiftnum, right.slotuse, 0);
        memcopy!(right, right, childid, shiftnum, right.slotuse + 1, 0);

        right.slotuse -= shiftnum;
    }

    fn shift_right_inner(
        left: &mut InnerNode<T>,
        right: &mut InnerNode<T>,
        parent: &mut InnerNode<T>,
        parentslot: u16,
    ) {
        debug_assert!(left.level == right.level);
        debug_assert!(parent.level == left.level + 1);
        debug_assert!(left.slotuse > right.slotuse);
        debug_assert!(
            unsafe { &*parent.get_child(parentslot as usize) }.as_inner() as *const InnerNode<T>
                as *mut InnerNode<T>
                == left as *mut InnerNode<T>
        );

        let shiftnum = (left.slotuse - right.slotuse) >> 1;

        log::debug!(
            "Shifting(inner) {} entries to right {:p} from left {:p} with common parent {:p}.",
            shiftnum,
            right,
            left,
            parent,
        );

        if T::SELF_VERIFY {
            let mut leftslot: u16 = 0;
            while leftslot <= parent.slotuse
                && unsafe { &*parent.get_child(leftslot as usize) }.as_inner()
                    as *const InnerNode<T> as *mut InnerNode<T>
                    != (left as *mut InnerNode<T>)
            {
                leftslot += 1;
            }

            debug_assert!(leftslot < parent.slotuse);
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize) }.as_inner() as *const InnerNode<T>
                    as *mut InnerNode<T>
                    == left as *mut InnerNode<T>,
            );
            debug_assert!(
                unsafe { &*parent.get_child(leftslot as usize + 1) }.as_inner()
                    as *const InnerNode<T> as *mut InnerNode<T>
                    == right as *mut InnerNode<T>,
            );
            debug_assert!(leftslot == parentslot);
        }
        // shift all slots in the right node

        debug_assert!((right.slotuse as usize + shiftnum as usize) < T::INNER_SLOTMAX);

        memcopy!(right, right, slotkey, 0, right.slotuse, shiftnum);
        memcopy!(right, right, childid, 0, right.slotuse + 1, shiftnum);

        right.slotuse += shiftnum;

        // copy the parent's decision slotkey and childid to the last new key on
        // the right
        right.slotkey[shiftnum as usize - 1] =
            MaybeUninit::new(parent.key(parentslot as usize).clone());

        // copy the remaining last items from the left node to the first slot in
        // the right node.
        memcopy!(
            left,
            right,
            slotkey,
            left.slotuse - shiftnum + 1,
            left.slotuse,
            0
        );
        // right.slotkey[0..shiftnum as usize - 1].copy_from_slice(
        //     &left.slotkey[(left.slotuse - shiftnum + 1) as usize..left.slotuse as usize],
        // );
        memcopy!(
            left,
            right,
            childid,
            left.slotuse - shiftnum + 1,
            left.slotuse + 1,
            0
        );
        // right.childid[0..shiftnum as usize].copy_from_slice(
        //     &left.childid[(left.slotuse - shiftnum + 1) as usize..left.slotuse as usize + 1],
        // );

        // copy the first to-be-removed key from the left node to the parent's
        // decision slot
        parent.slotkey[parentslot as usize] =
            MaybeUninit::new(left.key((left.slotuse - shiftnum) as usize).clone());

        left.slotuse -= shiftnum;
    }
}

/// Debug
impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn print_leaves(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "leaves:")?;
        let mut n = self.head_leaf_;

        while !n.is_null() {
            write!(f, " {:p} ", unsafe { &*n })?;
            n = unsafe { &*n }.next_leaf;
        }

        Ok(())
    }
    fn print_node(
        f: &mut std::fmt::Formatter<'_>,
        node: *mut Node<T>,
        depth: usize,
        recursive: bool,
    ) -> std::fmt::Result {
        for _ in 0..depth {
            write!(f, "  ")?;
        }

        let _n = unsafe { &*node };
        write!(
            f,
            "node {:p} level {} slotuse {}\n",
            _n, _n.level, _n.slotuse
        )?;

        if _n.is_leafnode() {
            let leafnode = _n.as_leaf();

            for _ in 0..depth {
                write!(f, "  ")?;
            }
            write!(
                f,
                "  leaf prev {:p} next {:p}\n",
                leafnode.prev_leaf, leafnode.next_leaf
            )?;

            for _ in 0..depth {
                write!(f, "  ")?;
            }

            for i in 0..leafnode.slotuse {
                write!(
                    f,
                    // " {:?} (data: {:?})",
                    " {:?}",
                    leafnode.key(i as usize),
                    // leafnode.slotdata[i as usize]
                )?;
            }
            write!(f, "\n")?;
        } else {
            let innernode: &InnerNode<T> = _n.as_inner();

            for _ in 0..depth {
                write!(f, "  ")?;
            }

            for i in 0..innernode.slotuse {
                write!(f, "({:p}) {:?} ", innernode.get_child(i as usize), unsafe {
                    innernode.slotkey[i as usize].assume_init_ref()
                })?;
            }
            write!(
                f,
                "({:p})\n",
                innernode.get_child(innernode.slotuse as usize)
            )?;

            if recursive {
                for i in 0..innernode.slotuse + 1 {
                    Self::print_node(f, innernode.get_child(i as usize), depth + 1, recursive)?;
                }
            }
        }

        Ok(())
    }
}

/// Display
impl<T: BTreeParams> std::fmt::Debug for BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.root_.is_null() {
            Self::print_node(f, self.root_, 0, true)?;
            writeln!(f)?;
            self.print_leaves(f)?;
        }

        Ok(())
    }
}

impl<T: BTreeParams> BTree<T>
where
    [(); T::LEAF_SLOTMAX]: Sized,
    [(); T::INNER_SLOTMAX + 1]: Sized,
{
    pub fn new() -> Self {
        Self {
            root_: null_mut(),
            head_leaf_: null_mut(),
            tail_leaf_: null_mut(),
            stats_: TreeStats::new(),
            key_less: T::KeyCompareType::new(),
        }
    }
}
