use embedded_alloc::Heap;

#[cfg(not(test))]
#[global_allocator]
pub(crate) static HEAP: Heap = Heap::empty();

#[cfg(test)]
pub(crate) static HEAP: Heap = Heap::empty();