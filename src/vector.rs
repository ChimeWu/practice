extern crate alloc;
use crate::slice::{AsSlice, SliceIter, SliceMutIter, SliceRaw, SliceRefIter};
use alloc::alloc::{AllocError, Allocator, Global, Layout, LayoutError};
use core::fmt::Display;
use core::{marker::PhantomData, ptr::NonNull};

#[derive(Debug)]
pub enum VectorError {
    LayoutError(LayoutError),
    AllocError(AllocError),
}

impl Display for VectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorError::LayoutError(e) => write!(f, "Layout error: {}", e),
            VectorError::AllocError(e) => write!(f, "Allocation error: {}", e),
        }
    }
}

impl From<LayoutError> for VectorError {
    fn from(e: LayoutError) -> Self {
        VectorError::LayoutError(e)
    }
}
impl From<AllocError> for VectorError {
    fn from(e: AllocError) -> Self {
        VectorError::AllocError(e)
    }
}

pub struct Vector<T> {
    slice_raw: SliceRaw<T>,
    capacity: usize,
    allocator: Global,
    _marker: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Self {
            slice_raw: SliceRaw::zst_slice(),
            capacity: 0,
            allocator: Global,
            _marker: PhantomData,
        }
    }
    pub fn with_capacity(capacity: usize) -> Result<Self, VectorError> {
        let allocator = Global;
        let layout = Layout::array::<T>(capacity)?;
        let buffer = if layout.size() > 0 {
            allocator.allocate(layout)?.cast()
        } else {
            NonNull::dangling()
        };
        Ok(Self {
            slice_raw: SliceRaw::new(buffer, 0),
            capacity,
            allocator,
            _marker: PhantomData,
        })
    }
    pub fn reserve(&mut self, additional: usize) -> Result<(), VectorError> {
        if self.len() + additional > self.capacity {
            let old_layout = Layout::array::<T>(self.capacity)?;
            let new_capacity = (self.capacity + additional).next_power_of_two();
            if new_capacity == 0 {
                return Ok(());
            }
            let new_layout = Layout::array::<T>(new_capacity)?;
            let new_buffer = unsafe {
                self.allocator
                    .grow(self.slice_raw.head().cast(), old_layout, new_layout)?
                    .cast()
            };
            *self.slice_raw.head_mut() = new_buffer;
            self.capacity = new_capacity;
            Ok(())
        } else {
            Ok(())
        }
    }
    pub fn shrink(&mut self) -> Result<(), VectorError> {
        if self.len() < self.capacity {
            let old_layout = Layout::array::<T>(self.capacity)?;
            let new_layout = Layout::array::<T>(self.len())?;
            let new_buffer = unsafe {
                self.allocator
                    .shrink(self.slice_raw.head().cast(), old_layout, new_layout)?
                    .cast()
            };
            *self.slice_raw.head_mut() = new_buffer;
            self.capacity = self.len();
            Ok(())
        } else {
            Ok(())
        }
    }
    pub fn push(&mut self, value: T) -> Result<(), VectorError> {
        self.reserve(1)?;
        unsafe {
            std::ptr::write(self.slice_raw.head().as_ptr().add(self.len()), value);
        }
        *self.slice_raw.len_mut() += 1;
        Ok(())
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.len() == 0 {
            None
        } else {
            *self.slice_raw.len_mut() -= 1;
            unsafe {
                Some(std::ptr::read(
                    self.slice_raw.head().as_ptr().add(self.len()),
                ))
            }
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice_ref().get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_slice_mut().get_mut(index)
    }
    pub fn iter(&self) -> SliceRefIter<'_, T> {
        self.as_slice_ref().into_iter()
    }
    pub fn iter_mut(&mut self) -> SliceMutIter<'_, T> {
        self.as_slice_mut().into_iter()
    }
    pub fn len(&self) -> usize {
        self.slice_raw.len()
    }
}

impl<T> Drop for Vector<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
        if self.capacity > 0 {
            unsafe {
                let layout = Layout::array::<T>(self.capacity).unwrap_unchecked();
                self.allocator
                    .deallocate(self.slice_raw.head().cast(), layout);
            }
        }
    }
}
impl<T> AsSlice for Vector<T> {
    type Target = T;
    fn as_slice_raw(&self) -> &SliceRaw<Self::Target> {
        &self.slice_raw
    }
}
impl<'a, T> IntoIterator for &'a Vector<T> {
    type Item = &'a T;
    type IntoIter = SliceRefIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Vector<T> {
    type Item = &'a mut T;
    type IntoIter = SliceMutIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct VectorIter<T> {
    _vector: Vector<T>,
    iter: SliceIter<T>,
}
impl<T> Iterator for VectorIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ptr| unsafe { ptr.read() })
    }
}
impl<T> IntoIterator for Vector<T> {
    type Item = T;
    type IntoIter = VectorIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        let iter = self.slice_raw.into_iter();
        VectorIter {
            _vector: self,
            iter,
        }
    }
}
impl<T: Clone> Clone for Vector<T> {
    fn clone(&self) -> Self {
        let mut new_vector = Self::with_capacity(self.len()).unwrap();
        for item in self.iter() {
            new_vector.push(item.clone()).unwrap();
        }
        new_vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vector() {
        let mut vec = Vector::new();
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        assert_eq!(vec.len(), 10);
        *vec.as_slice_mut().get_mut(2).unwrap() = 42;
        assert_eq!(vec.get(2).unwrap(), &42);
        // assert_eq!(vec.binary_search(&42), Some(2));
    }
    #[test]
    fn test_vector_iter() {
        let mut vec = Vector::new();
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        for i in vec.iter() {
            println!("{}", *i);
        }
        vec.iter_mut().for_each(|i| {
            *i = &*i + 2;
        });
        for i in vec.iter() {
            println!("{}", *i);
        }
    }
}
