use std::{mem::MaybeUninit, ptr::null_mut};

use super::btree_traits::{BTreeParams, InnerValueType, KeyOfValue};

pub trait NodeImpl<Tree: BTreeParams> {
    fn slotuse(&self) -> u16;
    fn key(&self, slot: usize) -> &Tree::KeyType;
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Node<Tree: BTreeParams>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    pub level: u16,
    pub slotuse: u16,
    marker: std::marker::PhantomData<Tree>,
}

#[derive(Debug)]
#[repr(C)]
pub struct InnerNode<Tree: BTreeParams>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    pub level: u16,
    pub slotuse: u16,
    pub slotkey: [MaybeUninit<Tree::KeyType>; Tree::INNER_SLOTMAX],
    pub childid: [MaybeUninit<*mut Node<Tree>>; Tree::INNER_SLOTMAX + 1],
}

#[derive(Debug)]
#[repr(C)]
pub struct LeafNode<Tree: BTreeParams>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    pub level: u16,
    pub slotuse: u16,
    pub prev_leaf: *mut Self,
    pub next_leaf: *mut Self,
    pub slotdata: [MaybeUninit<InnerValueType<Tree>>; Tree::LEAF_SLOTMAX],
}

impl<Tree: BTreeParams> Node<Tree>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    pub fn new_inner(level: u16) -> *mut Self {
        Box::into_raw(Box::new({
            InnerNode {
                level,
                slotuse: 0,
                slotkey: unsafe { MaybeUninit::zeroed().assume_init() },
                childid: unsafe { MaybeUninit::zeroed().assume_init() },
            }
        })) as *mut Self
    }

    pub fn new_leaf() -> *mut Self {
        Box::into_raw(Box::new({
            LeafNode {
                level: 0,
                slotuse: 0,
                prev_leaf: null_mut(),
                next_leaf: null_mut(),
                slotdata: unsafe { MaybeUninit::zeroed().assume_init() },
            }
        })) as *mut Self
    }

    #[inline]
    pub fn as_leaf_ptr(&self) -> *mut LeafNode<Tree> {
        self as *const Node<Tree> as *mut LeafNode<Tree>
    }

    #[inline]
    pub fn as_inner_ptr(&self) -> *mut InnerNode<Tree> {
        self as *const Node<Tree> as *mut InnerNode<Tree>
    }

    #[inline]
    pub fn as_leaf(&self) -> &LeafNode<Tree> {
        unsafe { &*self.as_leaf_ptr() }
    }

    #[inline]
    pub fn as_inner(&self) -> &InnerNode<Tree> {
        unsafe { &*self.as_inner_ptr() }
    }

    #[inline]
    pub fn as_leaf_mut(&mut self) -> &mut LeafNode<Tree> {
        unsafe { &mut *self.as_leaf_ptr() }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> &mut InnerNode<Tree> {
        unsafe { &mut *self.as_inner_ptr() }
    }

    #[inline]
    pub fn is_leafnode(&self) -> bool {
        self.level == 0
    }
}

impl<Tree: BTreeParams> InnerNode<Tree>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    #[inline]
    pub fn as_node_ptr(&self) -> *mut Node<Tree> {
        self as *const InnerNode<Tree> as *mut Node<Tree>
    }

    #[inline]
    pub fn as_node(&self) -> &Node<Tree> {
        unsafe { &*(self.as_node_ptr()) }
    }

    pub fn initialize(&mut self, l: u16) {
        self.level = l;
        self.slotuse = 0;
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        (self.slotuse as usize) == Tree::INNER_SLOTMAX
    }

    #[inline]
    pub fn is_few(&self) -> bool {
        (self.slotuse as usize) <= Tree::INNER_SLOTMIN
    }

    #[inline]
    pub fn is_underflow(&self) -> bool {
        (self.slotuse as usize) < Tree::INNER_SLOTMIN
    }

    #[inline]
    pub fn get_child(&self, slot: usize) -> *mut Node<Tree> {
        unsafe { self.childid[slot].assume_init() }
    }
}

impl<Tree: BTreeParams> LeafNode<Tree>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    #[inline]
    pub fn as_node_ptr(&self) -> *mut Node<Tree> {
        self as *const LeafNode<Tree> as *mut Node<Tree>
    }

    #[inline]
    pub fn as_node(&self) -> &Node<Tree> {
        unsafe { &*(self.as_node_ptr()) }
    }

    #[inline]
    pub fn initialize(&mut self) {
        self.level = 0;
        self.slotuse = 0;
        self.prev_leaf = null_mut();
        self.next_leaf = null_mut();
    }

    #[inline]
    pub fn set_slot(&mut self, slot: u16, value: InnerValueType<Tree>) {
        self.slotdata[slot as usize] = MaybeUninit::new(value);
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        (self.slotuse as usize) == Tree::LEAF_SLOTMAX
    }

    #[inline]
    pub fn is_few(&self) -> bool {
        (self.slotuse as usize) <= Tree::LEAF_SLOTMIN
    }

    #[inline]
    pub fn is_underflow(&self) -> bool {
        (self.slotuse as usize) < Tree::LEAF_SLOTMIN
    }
}

impl<Tree: BTreeParams> NodeImpl<Tree> for LeafNode<Tree>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    #[inline]
    fn slotuse(&self) -> u16 {
        self.slotuse
    }

    #[inline]
    fn key(&self, slot: usize) -> &Tree::KeyType {
        Tree::KeyOfValueType::get(unsafe { self.slotdata[slot].assume_init_ref() })
    }
}

impl<Tree: BTreeParams> NodeImpl<Tree> for InnerNode<Tree>
where
    [(); Tree::INNER_SLOTMAX + 1]: Sized,
    [(); Tree::LEAF_SLOTMAX]: Sized,
{
    #[inline]
    fn slotuse(&self) -> u16 {
        self.slotuse
    }

    #[inline]
    fn key(&self, slot: usize) -> &Tree::KeyType {
        unsafe { self.slotkey[slot].assume_init_ref() }
    }
}
