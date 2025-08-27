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

pub struct Vector<T, A: Allocator = Global> {
    buffer: NonNull<T>,
    capacity: usize,
    length: usize,
    allocator: A,
    _marker: PhantomData<T>,
}

impl<T> Vector<T, Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
    pub fn with_capacity(capacity: usize) -> Result<Self, VectorError> {
        Self::with_capacity_in(capacity, Global)
    }
}

impl<T, A: Allocator> Vector<T, A> {
    pub fn new_in(allocator: A) -> Self {
        Self {
            buffer: NonNull::dangling(),
            capacity: 0,
            length: 0,
            allocator,
            _marker: PhantomData,
        }
    }
    pub fn with_capacity_in(capacity: usize, allocator: A) -> Result<Self, VectorError> {
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

impl<T, A: Allocator> Deref for Vector<T, A> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr(), self.length) }
    }
}

impl<T, A: Allocator> DerefMut for Vector<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.buffer.as_ptr(), self.length) }
    }
}

impl<T, A: Allocator> Drop for Vector<T, A> {
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

impl<T: Clone, A: Allocator + Clone> Clone for Vector<T, A> {
    fn clone(&self) -> Self {
        let mut new_vector = Self::with_capacity_in(self.length, self.allocator.clone()).unwrap();
        for item in self.iter() {
            new_vector.push(item.clone()).unwrap();
        }
        new_vector
    }
}

impl<T: PartialEq, A: Allocator> PartialEq for Vector<T, A> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
impl<T: Eq, A: Allocator> Eq for Vector<T, A> {}

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
