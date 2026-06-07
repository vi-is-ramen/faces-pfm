bitflags::bitflags! {
    /// Page frame status flags.
    ///
    /// These flags track the state of a physical page frame, such as whether it is
    /// locked, dirty, up-to-date, part of the LRU list, etc. They are used by the
    /// page frame manager and compatible with the `faces::AbsFlags` trait.
    #[derive(Debug, Copy, Default, Eq, PartialEq)]
    pub struct PageFlags: u16 {
        /// Page is locked and cannot be evicted or moved.
        const LOCKED   = 1 << 0;
        /// Page has been written to (needs flushing to backing store).
        const DIRTY    = 1 << 1;
        /// Page content is up-to-date with its backing store.
        const UPTODATE = 1 << 2;
        /// Page is on the LRU (least recently used) list.
        const LRU      = 1 << 3;
        /// Page is a small (order‑0) allocation.
        const SMALL    = 1 << 4;
        /// Page is part of a compound page (e.g., huge page).
        const COMPOUND = 1 << 5;
        /// Page is in the swap cache.
        const SWAPCACHE= 1 << 6;
        /// Page is currently being written back to disk.
        const WRITEBACK= 1 << 7;
        /// Page is used as a page table.
        const PAGETABLE= 1 << 8;
        /// Page belongs to a file mapping.
        const FILE     = 1 << 9;
        /// Page is reserved (e.g., used by kernel or unavailable).
        const RESERVED = 1 << 10;
    }
}

impl const Clone for PageFlags {
    /// Clones the page flags by copying the underlying integer.
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl From<usize> for PageFlags {
    /// Converts a `usize` value into a `PageFlags` instance.
    ///
    /// # Safety
    /// This transmutes the integer representation directly. The caller must
    /// ensure that the `usize` contains a valid bit pattern for `PageFlags`.
    fn from(value: usize) -> Self {
        * unsafe {
            core::mem::transmute::<&usize, &Self>(&value)
        }
    }
}

impl faces::AbsFlags for PageFlags {}
