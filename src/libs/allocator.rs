use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{
    AtomicUsize,
    Ordering::{Acquire, SeqCst},
};

// ! Using a basic bump allocator, switch to something like a buddy allocator soon :tm:
const ARENA_SIZE: usize = 128 * 1024;
const MAX_SUPPORTED_ALIGN: usize = 4096;

#[repr(C, align(4096))]
struct SimpleAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remaining: AtomicUsize,
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remaining: AtomicUsize::new(ARENA_SIZE),
};

unsafe impl Sync for SimpleAllocator {}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // `Layout` contract forbids making a `Layout` with align=0, or align not power of 2.
        // So we can safely use a mask to ensure alignment without worrying about UB.
        let align_mask_to_round_down = !(align - 1);

        if align > MAX_SUPPORTED_ALIGN {
            return ptr::null_mut();
        }

        let mut allocated = 0;
        if self
            .remaining
            .fetch_update(SeqCst, SeqCst, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return ptr::null_mut();
        };
        self.arena.get().cast::<u8>().add(allocated)
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
