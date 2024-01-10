use std::ops;

use bitmask_enum::bitmask;

use super::btree_traits::BTreeParams;

#[bitmask(u8)]
pub enum DeletionResultFlags {
    Ok = 0,
    NotFound = 1,
    UpdateLastKey = 2,
    FixMerge = 4,
}

pub struct DeletionResult<T: BTreeParams> {
    pub flags: DeletionResultFlags,
    pub last_key: Option<T::KeyType>,
}

impl<T: BTreeParams> DeletionResult<T> {
    pub fn new(flags: DeletionResultFlags) -> Self {
        Self {
            flags,
            last_key: None,
        }
    }
    pub fn new_with_key(flags: DeletionResultFlags, last_key: T::KeyType) -> Self {
        Self {
            flags,
            last_key: Some(last_key),
        }
    }

    pub fn has(&self, flag: DeletionResultFlags) -> bool {
        self.flags.contains(flag)
    }
}

impl<T: BTreeParams> ops::BitOrAssign<DeletionResult<T>> for DeletionResult<T> {
    fn bitor_assign(&mut self, rhs: DeletionResult<T>) {
        self.flags = self.flags | rhs.flags;
        if rhs.has(DeletionResultFlags::UpdateLastKey) {
            self.last_key = rhs.last_key;
        }
    }
}
