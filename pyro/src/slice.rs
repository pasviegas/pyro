//! Temporary helper module until raw slices `*mut [T]` are on stable, or until `&[T]` is not UB
//! anymore for unitialized memory.
use std::marker::PhantomData;

pub enum Mutable {}
pub enum Immutable {}

mod sealed {
    pub trait Sealed {}
}
pub trait Mutability: sealed::Sealed {}
impl sealed::Sealed for Mutable {}
impl sealed::Sealed for Immutable {}

impl Mutability for Mutable {}
impl Mutability for Immutable {}

pub struct RawSlice<'a, M: Mutability, T> {
    pub start: *mut T,
    pub len: usize,
    _marker: PhantomData<&'a M>,
}

unsafe impl<M: Mutability, T: Send> Send for RawSlice<'_, M, T> {}
unsafe impl<M: Mutability, T: Sync> Sync for RawSlice<'_, M, T> {}

pub type Slice<'a, T> = RawSlice<'a, Immutable, T>;
pub type SliceMut<'a, T> = RawSlice<'a, Mutable, T>;

impl<'a, M, T> RawSlice<'a, M, T>
where
    M: Mutability,
{
    #[inline]
    pub unsafe fn get_unchecked(&self, idx: usize) -> *const T {
        self.start.add(idx) as *const T
    }
    #[inline]
    pub fn get(&self, idx: usize) -> *const T {
        assert!(idx < self.len);
        unsafe { self.get_unchecked(idx) }
    }
    #[inline]
    pub fn try_get(&self, idx: usize) -> Option<*const T> {
        let len = self.len;
        if idx >= len {
            return None;
        }
        Some(unsafe { self.get_unchecked(idx) })
    }
}

impl<'a, T> RawSlice<'a, Immutable, T> {
    pub fn split_at(self, idx: usize) -> (Self, Self) {
        unsafe {
            let left = Slice::from_raw(self.start, idx);
            let right = Slice::from_raw(self.start.add(idx), self.len - idx);
            (left, right)
        }
    }
    pub fn from_slice(slice: &'a [T]) -> Self {
        Self::from_raw(slice.as_ptr(), slice.len())
    }
    pub fn from_raw(start: *const T, len: usize) -> Self {
        Self {
            start: start as _,
            len,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> RawSlice<'a, Mutable, T> {
    #[inline]
    pub unsafe fn get_unchecked_mut(&self, idx: usize) -> *mut T {
        self.start.add(idx)
    }
    #[inline]
    pub fn get_mut(&self, idx: usize) -> *mut T {
        assert!(idx < self.len);
        unsafe { self.get_unchecked_mut(idx) }
    }
    #[inline]
    pub fn try_get_mut(&mut self, idx: usize) -> Option<*mut T> {
        let len = self.len;
        if idx >= len {
            return None;
        }
        Some(unsafe { self.get_unchecked_mut(idx) })
    }
    pub fn from_slice(slice: &'a mut [T]) -> Self {
        Self::from_raw(slice.as_mut_ptr(), slice.len())
    }
    pub fn from_raw(start: *mut T, len: usize) -> Self {
        Self {
            start,
            len,
            _marker: PhantomData,
        }
    }
    pub fn split_at_mut(self, idx: usize) -> (Self, Self) {
        unsafe {
            let left = SliceMut::from_raw(self.start, idx);
            let right = SliceMut::from_raw(self.start.add(idx), self.len - idx);
            (left, right)
        }
    }
}
