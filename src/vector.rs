extern crate alloc;
use alloc::alloc::{AllocError, Allocator, Global, Layout, LayoutError};
use core::fmt::Display;
use core::ops::{Deref, DerefMut};
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
    buffer: NonNull<T>,
    capacity: usize,
    length: usize,
    allocator: Global,
    _marker: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Self {
            buffer: NonNull::dangling(),
            capacity: 0,
            length: 0,
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
            buffer,
            capacity,
            length: 0,
            allocator,
            _marker: PhantomData,
        })
    }
    pub fn reserve(&mut self, additional: usize) -> Result<(), VectorError> {
        if self.length + additional > self.capacity {
            let old_layout = Layout::array::<T>(self.capacity)?;
            let new_capacity = (self.capacity + additional).next_power_of_two();
            if new_capacity == 0 {
                return Ok(());
            }
            let new_layout = Layout::array::<T>(new_capacity)?;
            let new_buffer = unsafe {
                self.allocator
                    .grow(self.buffer.cast(), old_layout, new_layout)?
                    .cast()
            };
            self.buffer = new_buffer;
            self.capacity = new_capacity;
            Ok(())
        } else {
            Ok(())
        }
    }
    pub fn shrink(&mut self) -> Result<(), VectorError> {
        if self.length < self.capacity {
            let old_layout = Layout::array::<T>(self.capacity)?;
            let new_layout = Layout::array::<T>(self.length)?;
            let new_buffer = unsafe {
                self.allocator
                    .shrink(self.buffer.cast(), old_layout, new_layout)?
                    .cast()
            };
            self.buffer = new_buffer;
            self.capacity = self.length;
            Ok(())
        } else {
            Ok(())
        }
    }
    pub fn push(&mut self, value: T) -> Result<(), VectorError> {
        self.reserve(1)?;
        unsafe {
            std::ptr::write(self.buffer.as_ptr().add(self.length), value);
        }
        self.length += 1;
        Ok(())
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            unsafe { Some(std::ptr::read(self.buffer.as_ptr().add(self.length))) }
        }
    }
}

impl<T> Deref for Vector<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr(), self.length) }
    }
}

impl<T> DerefMut for Vector<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.buffer.as_ptr(), self.length) }
    }
}

impl<T> Drop for Vector<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
        if self.capacity > 0 {
            unsafe {
                let layout = Layout::array::<T>(self.capacity).unwrap_unchecked();
                self.allocator.deallocate(self.buffer.cast(), layout);
            }
        }
    }
}

impl<T: Clone> Clone for Vector<T> {
    fn clone(&self) -> Self {
        let mut new_vector = Self::with_capacity(self.length).unwrap();
        for item in self.iter() {
            new_vector.push(item.clone()).unwrap();
        }
        new_vector
    }
}

impl<T: PartialEq> PartialEq for Vector<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
impl<T: Eq> Eq for Vector<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vector() {
        let mut vec = Vector::new();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity, 0);
        vec.push(1).unwrap();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], 1);
        vec.push(2).unwrap();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[1], 2);
        assert_eq!(vec.pop(), Some(2));
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.pop(), Some(1));
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.pop(), None);
    }
}
