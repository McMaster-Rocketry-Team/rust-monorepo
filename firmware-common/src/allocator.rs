use embedded_alloc::Heap;

#[global_allocator]
pub(crate) static HEAP: Heap = Heap::empty();
