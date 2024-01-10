macro_rules! memcopy {
    ($from:ident, $to:ident, $field:ident, $begin:expr, $end:expr, $dbegin:expr) => {
        unsafe {
            if ($from as *const _) == ($to as *const _) {
                std::ptr::copy(
                    $from.$field[$begin as usize..$end as usize].as_ptr(),
                    $from.$field[$dbegin as usize..].as_mut_ptr(),
                    ($end - $begin) as usize,
                )
            } else {
                std::ptr::copy_nonoverlapping(
                    $from.$field[$begin as usize..$end as usize].as_ptr(),
                    $to.$field[$dbegin as usize..].as_mut_ptr(),
                    ($end - $begin) as usize,
                )
            }
        }
    };
}

pub(crate) use memcopy;
