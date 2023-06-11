use embedded_alloc::Heap;

#[cfg(all(not(test), feature = "global-allocator"))]
#[global_allocator]
pub(crate) static HEAP: Heap = Heap::empty();

#[cfg(not(all(not(test), feature = "global-allocator")))]
pub(crate) static HEAP: Heap = Heap::empty();
