extern crate alloc;
use crate::slice::{
    AsSlice, SliceIter, SliceMutExt, SliceMutIter, SlicePtr, SliceRefExt, SliceRefIter,
};
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
    slice_raw: SlicePtr<T>,
    capacity: usize,
    allocator: Global,
    _marker: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Self {
            slice_raw: SlicePtr::zst_slice(),
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
            slice_raw: unsafe { SlicePtr::new(buffer, 0) },
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
impl<T> AsSlice<T> for Vector<T> {
    unsafe fn slice_ptr(&self) -> SlicePtr<T> {
        self.slice_raw
    }
}
impl<T> SliceRefExt<T> for Vector<T> {}
impl<T> SliceMutExt<T> for Vector<T> {}
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
    fn test_vector_push() {
        let mut vec = Vector::new();
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        assert_eq!(vec.len(), 10);
        *vec.get_mut(2).unwrap() = 42;
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
    #[test]
    fn test_vector_reverse() {
        let mut vec = Vector::new();
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        vec.reverse();
        for i in vec.iter() {
            println!("{}", *i);
        }
        for i in vec.iter().rev() {
            println!("{}", *i);
        }
    }
    #[test]
    fn test_vector_ref_and_mut() {
        let mut vec = Vector::new();
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        let mut vec_mut = vec.slice_mut();
        let vec_mut2 = vec_mut.range_mut(2..5);
        vec_mut.iter_mut().for_each(|i| {
            *i = *i + 2;
        });
        vec_mut2.iter_mut().for_each(|i| {
            *i = *i + 3;
        });
        let mut vec2 = (0..10).collect::<Vec<usize>>();
        let vec2_mut = vec2.as_mut_slice();
        let vec2_mut2 = &mut vec2_mut[2..5];
        vec2_mut.iter_mut().for_each(|i| {
            *i = *i + 2;
        });
        vec2_mut2.iter_mut().for_each(|i| {
            *i = *i + 3;
        });
    }
    // #[test]
    // fn test_vector_sort() {
    //     let mut vec = Vector::new();
    //     vec.push(3).unwrap();
    //     vec.push(1).unwrap();
    //     vec.push(4).unwrap();
    //     vec.push(1).unwrap();
    //     vec.push(5).unwrap();
    //     vec.push(9).unwrap();
    //     vec.push(2).unwrap();
    //     vec.push(6).unwrap();
    //     vec.push(5).unwrap();
    //     vec.push(3).unwrap();
    //     vec.push(5).unwrap();
    //     vec.sort();
    //     for i in vec.iter() {
    //         println!("{}", *i);
    //     }
    // }
}
