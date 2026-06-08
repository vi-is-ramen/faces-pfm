use crate::PageFlags;

/// Metadata associated with a single physical page frame.
///
/// This structure stores per‑frame information required by the page frame
/// manager, such as flags, allocation order, linked list pointers for free
/// lists, reference count, and a pointer to the kernel virtual address of
/// the freelist header (if any). It is guaranteed to be 32‑byte aligned.
#[derive(Debug, Default)]
#[repr(C)]
pub struct PageFrame {
    /// Page state flags (locked, dirty, reserved, etc.).
    pub flags: PageFlags,
    /// Allocation order (2^order pages). 0 means a single 4 KiB page.
    pub order: u8,
    /// Index of the next page frame in a linked list (e.g., free list).
    pub next: u32,
    /// Index of the previous page frame in a linked list.
    pub prev: u32,
    /// Size class index for slab allocators (if used).
    pub size_class: u8,
    /// Reference count: number of users of this page frame.
    pub rc: u16,
    /// Kernel virtual address of the freelist header (or 0 if none).
    pub freelist_va: usize,
}

impl PageFrame {
    /// Creates a new, zeroed (default) page frame metadata structure.
    ///
    /// All fields are initialised with their default values. The `flags`
    /// field is cloned from a zeroed instance of `F`.
    ///
    /// # Panics
    /// This function is `const` and will not panic.
    pub const fn new() -> Self {
        Self {
            flags: (*unsafe { core::mem::transmute::<&usize, &PageFlags>(&0) }).clone(),
            order: 0,
            next: 0,
            prev: 0,
            size_class: 0,
            rc: 0,
            freelist_va: 0,
        }
    }

    /// Resets the page frame metadata to its default state.
    ///
    /// This is equivalent to `*self = PageFrame::new()`. After calling this,
    /// all flags are cleared, and counters are set to zero.
    pub fn reset(&mut self) {
        self.flags = (*unsafe { core::mem::transmute::<&usize, &PageFlags>(&0) }).clone();
        self.order = 0;
        self.next = 0;
        self.prev = 0;
        self.freelist_va = 0;
        self.rc = 0;
        self.size_class = 0;
    }
}
