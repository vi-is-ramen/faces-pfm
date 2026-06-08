//! Page frame manager implementation for `faces`.
//!
//! self module provides a concrete singleton manager (`PFM`) that implements
//! `faces::AbsPageFrameManager`. It uses a static array of `spin::Mutex<PageFrame>`
//! to store per‑frame metadata, initialised from the memory map provided by the
//! Limine boot protocol.

use std::marker::PhantomData;

use crate::{PageFlags, PageFrame};
use faces::{AbsPageFrameManager, Convertable as _, PhysicalAddress, to, AbsAddressTranslator};
use log::{debug, info};

/// The page frame manager singleton.
///
/// self type implements `AbsPageFrameManager` and is the central point for
/// manipulating page frame flags and accessing per‑frame metadata.
#[derive(Debug, Copy, Clone, Default)]
pub struct PageFrameManager<T: AbsAddressTranslator> {
    phantom: PhantomData<T>,
}

type Fa = &'static mut [spin::Mutex<PageFrame>];

/// Static slice of locked page frame metadata.
///
/// Each entry is a `spin::Mutex<PageFrame>` protecting the per‑frame data.
/// The slice is initialised during `PageFrameManager::init()` and must be
/// considered immutable afterwards.
static mut FRAME_ARRAY: &mut [spin::Mutex<PageFrame>] = Fa::default();

/// Helper macro to obtain a locked guard for a given physical frame number.
///
/// # Safety
/// self macro dereferences `FRAME_ARRAY` mutably and must only be used after
/// the array has been initialised by `PageFrameManager::init()`.
macro_rules! frame { ($pfn:expr) => {unsafe{FRAME_ARRAY[to($pfn)].lock()}} }

/// Limine memory map request structure.
///
/// self forces the bootloader to provide a complete memory map, which is used
/// to determine usable RAM and to allocate the frame metadata array.
#[used]
#[unsafe(link_section = ".requests")]
static MMAP: limine::request::MemmapRequest =  limine::request::MemmapRequest::new();

impl<T: AbsAddressTranslator> PageFrameManager<T> {
    /// Creates a new uninitialised page frame manager.
    ///
    /// The manager is not usable until `init()` is called.
    pub const fn new() -> Self { Self { phantom: PhantomData::<T>::default() } }

    /// Detects the total amount of physical memory from the Limine memory map.
    ///
    /// Sums the lengths of all memory map entries. self is used to calculate
    /// the total number of 4 KiB page frames.
    fn detect_total_physical_memory() -> usize {
        let memmap = MMAP.response().expect("Failed to obtain memory map").entries();
        let mut total = 0;
        for entry in memmap {
            total += entry.length as usize;
        }
        total
    }

    /// Finds a suitable region of physical memory to place the frame metadata array.
    ///
    /// # Arguments
    /// * `num_frames` - Number of `PageFrame` entries to allocate.
    ///
    /// # Returns
    /// A tuple `(base_physical_address, size_in_bytes)` of the chosen region.
    ///
    /// # Panics
    /// Panics if no usable memory region of sufficient size is found.
    fn find_better_place(num_frames: usize) -> (PhysicalAddress, usize) {
        let memmap = MMAP.response().expect("Failed to obtain memory map").entries();
        let required_size = (num_frames * core::mem::size_of::<spin::Mutex<PageFrame>>()).next_multiple_of(4096);
        let mut best_start = 0;
        let mut best_len = 0;

        for entry in memmap {
            if entry.type_ != limine::memmap::MEMMAP_USABLE {
                continue;
            }
            let len = entry.length as usize;
            if len >= required_size && len > best_len {
                best_len = len;
                best_start = entry.base;
            }
        }
        if best_len == 0 {
            panic!("No suitable memory region for PageFrameInfo array (need {} bytes)", required_size);
        }
        (to(best_start as usize), required_size)
    }

    /// Initialises the global page frame manager.
    ///
    /// self function:
    /// 1. Detects total physical memory and computes the number of frames.
    /// 2. Finds a physical memory region large enough to hold the `PageFrame` array.
    /// 3. Maps that region into the kernel’s virtual address space.
    /// 4. Initialises each `PageFrame` with default values.
    /// 5. Sets the `RESERVED` flag for all frames, then clears it for usable RAM entries.
    /// 6. Finally reserves the frames that contain the metadata array itself.
    ///
    /// # Panics
    /// Panics if the memory map is unavailable or if no suitable region is found.
    pub fn init(&self) {
        let total_memory = Self::detect_total_physical_memory();
        let num_frames = total_memory >> 12;
        let (phys_addr, size) = Self::find_better_place(num_frames);
        let virt_addr = T::as_virt(phys_addr);
        let addr: &mut spin::Mutex<PageFrame>;
        unsafe {
            addr = faces::unsafe_to(virt_addr);
        }

        debug!("Total mem: {:#x}", total_memory);
        debug!("Total frames: {}", num_frames);
        debug!("PFI array address: {:#x}", virt_addr.to());

        info!("pfm: Initializing page frame array");

        unsafe {
            let slice = core::slice::from_raw_parts_mut::<'static, spin::Mutex<PageFrame>>(addr, num_frames);
            for frame in slice.iter_mut() {
                *frame = spin::Mutex::<PageFrame>::new(PageFrame::new());
            }
            FRAME_ARRAY = slice;
        }

        info!("pfm: Obtaining memory map from bootloader");

        let memmap = MMAP.response().expect("Failed to obtain memory map").entries();

        info!("pfm: Reserving whole array");

        for idx in 0..num_frames {
            self.set_flags(to(idx), PageFlags::RESERVED);
        }

        info!("pfm: Searching usable memory");

        for entry in memmap {
            if entry.type_ == limine::memmap::MEMMAP_USABLE {
                let start = (entry.base / 4096) as usize;
                let end = ((entry.base + entry.length + 4095) / 4096) as usize;
                for idx in start..end.min(num_frames) {
                    self.clear_flags(to(idx), PageFlags::RESERVED);
                }
            }
        }

        info!("pfm: Reserving system memory areas");

        let array_start: PFN = to(to(phys_addr) >> 12);
        let array_end: PFN = to(to(array_start + to(size + 4095)) >> 12);
        for idx in array_start.to()..array_end.to().min(num_frames) {
            self.set_flags(to(idx), PageFlags::RESERVED);
        }
    }
}

/// Type alias for a physical frame number (PFN) as defined by `faces`.
type PFN = faces::PageFrameNumber;

impl<T: AbsAddressTranslator> AbsPageFrameManager for PageFrameManager<T> {
    type Flags = PageFlags;
    type Access = spin::MutexGuard<'static, PageFrame, spin::Spin>;

    /// Checks whether a given flag is set on the specified page frame.
    fn check_flags(&self, pfn: PFN, flag: PageFlags) -> bool {
        frame![pfn].flags & flag != PageFlags::empty()
    }

    /// Clears the specified flags on the given page frame.
    fn clear_flags(&self, pfn: PFN, flag: PageFlags) {
        frame![pfn].flags &= !flag
    }

    /// Returns the maximum possible page frame number.
    ///
    /// # TODO
    /// self currently returns `PFN::MAX`, which may be larger than the actual
    /// number of frames. A proper implementation should return the highest
    /// valid PFN based on the detected memory size.
    fn max(&self) -> PFN {
        to(usize::MAX)
    }

    /// Returns the minimum possible page frame number (always 0).
    fn min(&self) -> PFN {
        to(0)
    }

    /// Returns whether a page frame is physically present.
    ///
    /// # TODO
    /// self always returns `true`. A real implementation should validate
    /// the PFN against the number of available frames.
    fn present(&self, _pfn: PFN) -> bool {
        true
    }

    /// Sets the specified flags on the given page frame.
    fn set_flags(&self, pfn: PFN, flag: PageFlags) {
        frame!(pfn).flags |= flag
    }

    /// Returns a locked guard that provides mutable access to the page frame metadata.
    fn get(&self, pfn: PFN) -> Self::Access {
        frame!(pfn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PageFrame;
    use spin::Mutex;

    pub struct IDEN;

    impl AbsAddressTranslator for IDEN {
        fn as_phys(v: faces::VirtualAddress) -> PhysicalAddress {
            v.to().to()
        }

        fn as_virt(p: PhysicalAddress) -> faces::VirtualAddress {
            p.to().to()
        }
    }

    /// Create a mock frame array of given length and leak it to a 'static mut slice.
    /// Then assign it to FRAME_ARRAY for testing.
    unsafe fn setup_mock_frames(num_frames: usize) {
        // Build a vector of Mutex<PageFrame> initialised with default frames.
        let mut vec = Vec::with_capacity(num_frames);
        for _ in 0..num_frames {
            vec.push(Mutex::new(PageFrame::default()));
        }
        // Leak the vector to obtain a 'static mut slice.
        let slice: &'static mut [Mutex<PageFrame>] = Box::leak(vec.into_boxed_slice());
        unsafe {
            FRAME_ARRAY = slice;
        }
    }

    /// Reset FRAME_ARRAY to an empty slice after each test to avoid cross-test interference.
    unsafe fn teardown_mock_frames() {
        unsafe {
            FRAME_ARRAY = Fa::default();
        }
    }

    #[test]
    fn test_flag_operations() {
        unsafe {
            setup_mock_frames(10);
        }

        let pfm = PageFrameManager::<IDEN>::new();
        let pfn = faces::to(5);

        // Initially no flags should be set.
        assert!(!pfm.check_flags(pfn, PageFlags::LOCKED));
        assert!(!pfm.check_flags(pfn, PageFlags::DIRTY));

        // Set LOCKED flag.
        pfm.set_flags(pfn, PageFlags::LOCKED);
        assert!(pfm.check_flags(pfn, PageFlags::LOCKED));
        assert!(!pfm.check_flags(pfn, PageFlags::DIRTY));

        // Set DIRTY flag (should keep LOCKED).
        pfm.set_flags(pfn, PageFlags::DIRTY);
        assert!(pfm.check_flags(pfn, PageFlags::LOCKED));
        assert!(pfm.check_flags(pfn, PageFlags::DIRTY));

        // Clear LOCKED flag.
        pfm.clear_flags(pfn, PageFlags::LOCKED);
        assert!(!pfm.check_flags(pfn, PageFlags::LOCKED));
        assert!(pfm.check_flags(pfn, PageFlags::DIRTY));

        // Clear DIRTY flag.
        pfm.clear_flags(pfn, PageFlags::DIRTY);
        assert!(!pfm.check_flags(pfn, PageFlags::LOCKED));
        assert!(!pfm.check_flags(pfn, PageFlags::DIRTY));

        unsafe {
            teardown_mock_frames();
        }
    }

    #[test]
    fn test_get_frame_mut() {
        unsafe {
            setup_mock_frames(3);
        }

        let pfm = PageFrameManager::<IDEN>::new();
        let pfn = faces::to(1);

        // Obtain a guard and modify the frame directly.
        {
            let mut guard = pfm.get(pfn);
            guard.flags = PageFlags::SWAPCACHE;
            guard.order = 42;
            guard.rc = 100;
        }

        // Verify changes via flag checks.
        assert!(pfm.check_flags(pfn, PageFlags::SWAPCACHE));
        // Also verify using get and reading back.
        let guard = pfm.get(pfn);
        assert_eq!(guard.order, 42);
        assert_eq!(guard.rc, 100);

        unsafe {
            teardown_mock_frames();
        }
    }

    #[test]
    fn test_min_max_present() {
        let pfm = PageFrameManager::<IDEN>::new();
        // present currently always returns true
        assert!(pfm.present(faces::to(0)));
        assert!(pfm.present(faces::to(12345)));
    }

    #[test]
    fn test_frame_macro() {
        unsafe {
            setup_mock_frames(5);
        }

        let pfm = PageFrameManager::<IDEN>::new();

        let pfn: faces::PageFrameNumber = faces::to(2);
        // The macro frame! should give a locked guard.
        {
            let mut guard = frame!(pfn);
            guard.flags = PageFlags::COMPOUND;
        }
        assert!(pfm.check_flags(pfn, PageFlags::COMPOUND));

        unsafe {
            teardown_mock_frames();
        }
    }
}
