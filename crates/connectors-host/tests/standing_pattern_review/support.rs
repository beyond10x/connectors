//! The one counting allocator of this test binary. A binary has one global
//! allocator, so the review modules that measure what a read holds share
//! this one and read its two counters.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

/// Counts the bytes each thread holds and the most it held, so a test
/// measures its own thread whatever else runs in the process.
struct Counting;

thread_local! {
    /// The bytes this thread holds.
    pub(crate) static LIVE: Cell<isize> = const { Cell::new(0) };
    /// The most `LIVE` was since a test last set it.
    pub(crate) static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn held(by: isize) {
    let _ = LIVE.try_with(|live| {
        live.set(live.get() + by);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(live.get())));
    });
}

// SAFETY: every call is passed to the system allocator unchanged; the
// counters are thread-local cells.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is the system allocator's.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            held(layout.size() as isize);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is the system allocator's.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            held(layout.size() as isize);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is the system allocator's.
        unsafe { System.dealloc(ptr, layout) };
        held(-(layout.size() as isize));
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract is the system allocator's.
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            held(new_size as isize - layout.size() as isize);
        }
        moved
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;
