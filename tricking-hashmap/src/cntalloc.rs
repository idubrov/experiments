use std::alloc::{Opaque, GlobalAlloc, Global, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CountingAllocator;

thread_local! {
    static ALLOC_COUNTER: AtomicUsize = AtomicUsize::new(0);
}

impl CountingAllocator {
    pub fn allocations(&self) -> usize {
        ALLOC_COUNTER.with(|cnt| cnt.load(Ordering::Relaxed))
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut Opaque {
        ALLOC_COUNTER.with(|cnt| cnt.fetch_add(1, Ordering::Relaxed));
        System.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut Opaque, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut Opaque {
        ALLOC_COUNTER.with(|cnt| cnt.fetch_add(1, Ordering::Relaxed));
        System.alloc_zeroed(layout)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut Opaque, layout: Layout, new_size: usize) -> *mut Opaque {
        ALLOC_COUNTER.with(|cnt| cnt.fetch_add(1, Ordering::Relaxed));
        System.realloc(ptr, layout, new_size)
    }
}
