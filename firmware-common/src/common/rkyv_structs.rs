use core::ops::{Index, IndexMut};

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct RkyvVec<const N: usize, T: Copy + Default> {
    pub data: [T; N],
    pub len: usize,
}

impl<const N: usize, T: Copy + Default> RkyvVec<N, T> {
    pub fn from_slice(slice: &[T]) -> Self {
        let mut data = [Default::default(); N];
        let len = slice.len().min(N);
        data[..len].copy_from_slice(&slice[..len]);
        Self { data, len }
    }

    pub fn from_heapless_vec(slice: &heapless::Vec<T, N>) -> Self {
        Self::from_slice(&slice)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data[..self.len]
    }
}

impl<const N: usize, T: Copy + Default> Default for RkyvVec<N, T> {
    fn default() -> Self {
        Self {
            data: [Default::default(); N],
            len: Default::default(),
        }
    }
}

impl<const N: usize, T: Copy + Default> Index<usize> for RkyvVec<N, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<const N: usize, T: Copy + Default> IndexMut<usize> for RkyvVec<N, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

#[derive(Default, Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct RkyvString<const N: usize> {
    pub vec: RkyvVec<N, u8>,
}

impl<const N: usize> RkyvString<N> {
    pub fn from_str(s: &str) -> Self {
        Self {
            vec: RkyvVec::from_slice(s.as_bytes()),
        }
    }

    pub fn from_heapless_str(s: &heapless::String<N>) -> Self {
        Self::from_str(s.as_str())
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.vec.as_slice()).unwrap()
    }
}
