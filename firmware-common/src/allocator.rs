use embedded_alloc::Heap;

#[cfg(feature = "global-allocator")]
#[global_allocator]
pub(crate) static HEAP: Heap = Heap::empty();

#[cfg(not(feature = "global-allocator"))]
pub(crate) static HEAP: Heap = Heap::empty();
