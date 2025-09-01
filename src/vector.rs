extern crate alloc;
use alloc::alloc::{AllocError, Allocator, Global, Layout, LayoutError};
use core::fmt::Display;
use core::{marker::PhantomData, ptr::NonNull};
use std::ops::{Index, IndexMut, Range, RangeFull};

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
    len: usize,
    capacity: usize,
    allocator: Global,
    _marker: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Self {
            buffer: NonNull::dangling(),
            len: 0,
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
            buffer,
            len: 0,
            capacity,
            allocator,
            _marker: PhantomData,
        })
    }
    pub fn reserve(&mut self, additional: usize) -> Result<(), VectorError> {
        if self.len + additional > self.capacity {
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
        if self.len < self.capacity {
            let old_layout = Layout::array::<T>(self.capacity)?;
            let new_layout = Layout::array::<T>(self.len)?;
            let new_buffer = unsafe {
                self.allocator
                    .shrink(self.buffer.cast(), old_layout, new_layout)?
                    .cast()
            };
            self.buffer = new_buffer;
            self.capacity = self.len;
            Ok(())
        } else {
            Ok(())
        }
    }
    pub fn push(&mut self, value: T) -> Result<(), VectorError> {
        self.reserve(1)?;
        unsafe {
            std::ptr::write(self.buffer.as_ptr().add(self.len()), value);
        }
        self.len += 1;
        Ok(())
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(std::ptr::read(self.buffer.as_ptr().add(self.len()))) }
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            None
        } else {
            Some(unsafe { self.buffer.add(index).as_ref() })
        }
    }
    pub fn get_mut(&self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            None
        } else {
            Some(unsafe { self.buffer.add(index).as_mut() })
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn iter(&self) -> VecRefIter<'_, T> {
        self.into_iter()
    }
    pub fn iter_mut(&mut self) -> VecMutIter<'_, T> {
        self.into_iter()
    }
    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.buffer.as_ptr().cast_const(), self.len) }
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.buffer.as_ptr(), self.len) }
    }
    pub fn reverse(&mut self) {
        let raw = RawVecPtrRange::from(&*self);
        raw.reverse();
    }
    pub fn swap(&mut self, i: usize, j: usize) {
        let raw = RawVecPtrRange::from(&*self);
        raw.swap(i, j);
    }
    pub fn sort_by<F: Fn(&T, &T) -> core::cmp::Ordering + Copy>(&mut self, f: F) {
        let raw = RawVecPtrRange::from(&*self);
        raw.quick_sort_by(f);
    }
    pub fn sort_by_key<U: Ord, F: Fn(&T) -> &U>(&mut self, f: F) {
        let cmp_f = |t1: &T, t2: &T| f(t1).cmp(f(t2));
        self.sort_by(cmp_f);
    }
}

impl<T: Ord> Vector<T> {
    pub fn sort(&mut self) {
        self.sort_by(Ord::cmp);
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

pub struct VectorIter<T> {
    _vector: Vector<T>,
    iter: RawVecPtrRange<T>,
}
impl<T> Iterator for VectorIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ptr| unsafe { ptr.read() })
    }
}
impl<T> DoubleEndedIterator for VectorIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|ptr| unsafe { ptr.read() })
    }
}
impl<T> ExactSizeIterator for VectorIter<T> {
    fn len(&self) -> usize {
        self._vector.len
    }
}
impl<T> IntoIterator for Vector<T> {
    type Item = T;
    type IntoIter = VectorIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        VectorIter {
            iter: RawVecPtrRange::from(&self),
            _vector: self,
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

#[derive(Debug)]
struct RawVecPtrRange<T> {
    head: NonNull<T>,
    tail: NonNull<T>,
}
impl<T> RawVecPtrRange<T> {
    unsafe fn range(self, range: Range<usize>) -> Self {
        unsafe {
            Self {
                head: self.head.add(range.start),
                tail: self.tail.add(range.end),
            }
        }
    }
    unsafe fn split(self, index: usize) -> (Self, Self) {
        let split_at = unsafe { self.head.add(index) };
        (
            Self {
                head: self.head,
                tail: split_at,
            },
            Self {
                head: split_at,
                tail: self.tail,
            },
        )
    }
    pub fn len(self) -> usize {
        unsafe { self.tail.offset_from(self.head).unsigned_abs() }
    }
    pub fn swap(self, i: usize, j: usize) {
        let len = self.len();
        if i >= len || j >= len {
            panic!("index out of range")
        }
        if i == j {
            return;
        }
        unsafe {
            core::ptr::swap(self.head.add(i).as_ptr(), self.head.add(j).as_ptr());
        }
    }
    pub fn reverse(mut self) {
        while self.head < self.tail {
            unsafe {
                core::ptr::swap(self.head.as_ptr(), self.tail.sub(1).as_ptr());
                self.head = self.head.add(1);
                self.tail = self.tail.sub(1);
            }
        }
    }
    pub fn quick_sort_by<F: Fn(&T, &T) -> core::cmp::Ordering + Copy>(self, f: F) {
        let len = self.len();
        if len <= 1 {
            return;
        }
        let pivot_index = len - 1;
        let pivot = &self[pivot_index];
        let mut i = 0;
        for j in 0..pivot_index {
            if f(&self[j], pivot) == core::cmp::Ordering::Less {
                self.swap(i, j);
                i += 1;
            }
        }
        self.swap(i, pivot_index);
        let (left, right) = unsafe { self.split(i) };
        left.quick_sort_by(f);
        right.quick_sort_by(f);
    }
}
impl<T> Clone for RawVecPtrRange<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head,
            tail: self.tail,
        }
    }
}
impl<T> Copy for RawVecPtrRange<T> {}
impl<T> Iterator for RawVecPtrRange<T> {
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.head < self.tail {
            let res = self.head;
            self.head = unsafe { self.head.add(1) };
            Some(res)
        } else {
            None
        }
    }
}
impl<T> DoubleEndedIterator for RawVecPtrRange<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.tail = unsafe { self.tail.sub(1) };
        if self.tail >= self.head {
            Some(self.tail)
        } else {
            None
        }
    }
}
impl<T> ExactSizeIterator for RawVecPtrRange<T> {
    fn len(&self) -> usize {
        unsafe { self.tail.offset_from(self.head) }.abs() as usize
    }
}
impl<T> From<&Vector<T>> for RawVecPtrRange<T> {
    fn from(value: &Vector<T>) -> Self {
        RawVecPtrRange {
            head: value.buffer,
            tail: unsafe { value.buffer.add(value.len) },
        }
    }
}
impl<T> From<RawVecPtrRange<T>> for Range<*const T> {
    fn from(value: RawVecPtrRange<T>) -> Self {
        Range {
            start: value.head.as_ptr().cast_const(),
            end: value.tail.as_ptr().cast_const(),
        }
    }
}
impl<T> From<RawVecPtrRange<T>> for Range<*mut T> {
    fn from(value: RawVecPtrRange<T>) -> Self {
        Range {
            start: value.head.as_ptr(),
            end: value.tail.as_ptr(),
        }
    }
}
impl<T> Index<usize> for RawVecPtrRange<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            let ptr = self.head.add(index);
            if ptr >= self.tail {
                panic!("index out of bound");
            }
            ptr.as_ref()
        }
    }
}
impl<T> IndexMut<usize> for RawVecPtrRange<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            let mut ptr = self.head.add(index);
            if ptr >= self.tail {
                panic!("index out of bound");
            }
            ptr.as_mut()
        }
    }
}

pub struct VecRefIter<'a, T> {
    iter: RawVecPtrRange<T>,
    _marker: PhantomData<&'a T>,
}
impl<'a, T> Iterator for VecRefIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ptr| unsafe { ptr.as_ref() })
    }
}
impl<'a, T> DoubleEndedIterator for VecRefIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|ptr| unsafe { ptr.as_ref() })
    }
}
impl<'a, T> ExactSizeIterator for VecRefIter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T> From<RawVecPtrRange<T>> for VecRefIter<'a, T> {
    fn from(iter: RawVecPtrRange<T>) -> Self {
        Self {
            iter,
            _marker: PhantomData,
        }
    }
}
impl<'a, T> IntoIterator for &'a Vector<T> {
    type IntoIter = VecRefIter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        VecRefIter::from(RawVecPtrRange::from(self))
    }
}

pub struct VecMutIter<'a, T> {
    iter: RawVecPtrRange<T>,
    _marker: PhantomData<&'a mut T>,
}
impl<'a, T> Iterator for VecMutIter<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|mut ptr| unsafe { ptr.as_mut() })
    }
}
impl<'a, T> DoubleEndedIterator for VecMutIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|mut ptr| unsafe { ptr.as_mut() })
    }
}
impl<'a, T> ExactSizeIterator for VecMutIter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T> From<RawVecPtrRange<T>> for VecMutIter<'a, T> {
    fn from(iter: RawVecPtrRange<T>) -> Self {
        Self {
            iter,
            _marker: PhantomData,
        }
    }
}
impl<'a, T> IntoIterator for &'a mut Vector<T> {
    type Item = &'a mut T;
    type IntoIter = VecMutIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        VecMutIter::from(RawVecPtrRange::from(&*self))
    }
}

impl<T> Index<usize> for Vector<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("out bound of index")
    }
}
impl<T> IndexMut<usize> for Vector<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("out bound of index")
    }
}
impl<T> Index<Range<usize>> for Vector<T> {
    type Output = [T];
    fn index(&self, index: Range<usize>) -> &[T] {
        if index.start >= self.len || index.end > self.len {
            panic!("index out of bound!");
        }
        let iter = RawVecPtrRange::from(self);
        unsafe {
            let iter = iter.range(index);
            core::slice::from_ptr_range(iter.into())
        }
    }
}
impl<T> IndexMut<Range<usize>> for Vector<T> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        if index.start >= self.len || index.end > self.len {
            panic!("index out of bound!");
        }
        let iter = RawVecPtrRange::from(&*self);
        unsafe {
            let iter = iter.range(index);
            core::slice::from_mut_ptr_range(iter.into())
        }
    }
}
impl<T> Index<RangeFull> for Vector<T> {
    type Output = [T];
    fn index(&self, index: RangeFull) -> &Self::Output {
        let _ = index;
        self.as_slice()
    }
}
impl<T> IndexMut<RangeFull> for Vector<T> {
    fn index_mut(&mut self, index: RangeFull) -> &mut Self::Output {
        let _ = index;
        self.as_slice_mut()
    }
}

macro_rules! vector {
    () => {
        Vector::new()
    };
    ($elem:expr;$n:expr) =>{{
        let mut v = Vector::new();
        for i in 0..$n {
            v.push($elem).unwrap();
        }
        v
    }};
    ($($elem:expr),+ $(,)?) => {{
        let mut v = Vector::new();
        $(
            v.push($elem).unwrap();
        )+
        v
    }};
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
        for i in vec.into_iter() {
            println!("{}", i);
        }
        let mut vec = Vector::new();
        for i in 0..9 {
            vec.push(i).unwrap();
        }
        vec.reverse();
        for i in vec.into_iter() {
            println!("{}", i);
        }
    }
    #[test]
    fn test_vector_sort() {
        let mut vec = vector![3, 5, 8, 1, 2, 7, 4, 6];
        vec.sort();
        for i in vec {
            println!("{}", i);
        }
    }
}
