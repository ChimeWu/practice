use std::{
    marker::PhantomData,
    ops::{Deref, Index, IndexMut, Range},
    ptr::NonNull,
};

pub trait AsSlice {
    type Target;
    /// required method
    fn as_slice_raw(&self) -> &SliceRaw<Self::Target>;
    /// provided methods
    fn as_slice_ref<'a>(&'a self) -> SliceRef<'a, Self::Target> {
        SliceRef {
            ptr: *self.as_slice_raw(),
            _marker: PhantomData,
        }
    }
    fn as_slice_mut<'a>(&'a mut self) -> SliceMut<'a, Self::Target> {
        SliceMut {
            ptr: *self.as_slice_raw(),
            _marker: PhantomData,
        }
    }
    fn len(&self) -> usize {
        self.as_slice_raw().len()
    }
    fn slice_raw(&self, range: Range<usize>) -> SliceRaw<Self::Target> {
        assert!(range.start <= range.end && range.end <= self.len());
        if std::mem::size_of::<Self::Target>() == 0 {
            *self.as_slice_raw()
        } else {
            SliceRaw::new(
                unsafe { self.as_slice_raw().get(range.start).unwrap_unchecked() },
                range.end - range.start,
            )
        }
    }
    fn slice_ref<'a>(&'a self, range: Range<usize>) -> SliceRef<'a, Self::Target> {
        SliceRef {
            ptr: self.slice_raw(range),
            _marker: PhantomData,
        }
    }
    fn slice_mut<'a>(&'a mut self, range: Range<usize>) -> SliceMut<'a, Self::Target> {
        SliceMut {
            ptr: self.slice_raw(range),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliceRaw<T> {
    head: NonNull<T>,
    len: usize,
}
impl<T> SliceRaw<T> {
    pub fn zst_slice() -> Self {
        Self {
            head: NonNull::dangling(),
            len: 0,
        }
    }
    pub fn new(head: NonNull<T>, len: usize) -> Self {
        Self { head, len }
    }
    pub fn head(&self) -> NonNull<T> {
        self.head
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn head_mut(&mut self) -> &mut NonNull<T> {
        &mut self.head
    }
    pub fn len_mut(&mut self) -> &mut usize {
        &mut self.len
    }
    pub fn get(&self, index: usize) -> Option<NonNull<T>> {
        if index >= self.len {
            None
        } else {
            Some(unsafe { self.head.add(index) })
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
impl<T> Clone for SliceRaw<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head,
            len: self.len,
        }
    }
}
impl<T> Copy for SliceRaw<T> {}

#[derive(Debug, Clone)]
pub struct SliceIter<T> {
    head: NonNull<T>,
    tail: NonNull<T>,
}
impl<T> Iterator for SliceIter<T> {
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
impl<T> DoubleEndedIterator for SliceIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.tail = unsafe { self.tail.sub(1) };
        if self.tail >= self.head {
            Some(self.tail)
        } else {
            None
        }
    }
}
impl<T> ExactSizeIterator for SliceIter<T> {
    fn len(&self) -> usize {
        unsafe { self.tail.offset_from(self.head) }.abs() as usize
    }
}
impl<T> IntoIterator for SliceRaw<T> {
    type Item = NonNull<T>;
    type IntoIter = SliceIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        SliceIter {
            head: self.head,
            tail: unsafe { self.head.add(self.len) },
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliceRef<'a, T: 'a + Sized> {
    ptr: SliceRaw<T>,
    _marker: PhantomData<&'a T>,
}
impl<'a, T: 'a + Sized> SliceRef<'a, T> {
    pub fn get(&self, index: usize) -> Option<&'a T> {
        self.ptr.get(index).map(|ptr| unsafe { ptr.as_ref() })
    }
}
impl<'a, T: 'a + Sized> Clone for SliceRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T: 'a + Sized> Copy for SliceRef<'a, T> {}
impl<'a, T: 'a + Sized> Index<usize> for SliceRef<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

pub struct SliceRefIter<'a, T> {
    iter: SliceIter<T>,
    _marker: PhantomData<&'a T>,
}
impl<'a, T> Iterator for SliceRefIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ptr| unsafe { ptr.as_ref() })
    }
}
impl<'a, T> DoubleEndedIterator for SliceRefIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|ptr| unsafe { ptr.as_ref() })
    }
}
impl<'a, T> ExactSizeIterator for SliceRefIter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T> IntoIterator for SliceRef<'a, T> {
    type Item = &'a T;
    type IntoIter = SliceRefIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        SliceRefIter {
            iter: self.ptr.into_iter(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliceMut<'a, T: 'a + Sized> {
    ptr: SliceRaw<T>,
    _marker: PhantomData<&'a mut T>,
}
impl<'a, T: 'a + Sized> SliceMut<'a, T> {
    pub fn get_mut(&mut self, index: usize) -> Option<&'a mut T> {
        self.ptr.get(index).map(|mut ptr| unsafe { ptr.as_mut() })
    }
}
impl<'a, T: 'a + Sized> Clone for SliceMut<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T: 'a + Sized> Copy for SliceMut<'a, T> {}
impl<'a, T: 'a + Sized> Deref for SliceMut<'a, T> {
    type Target = SliceRef<'a, T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const SliceRef<T>) }
    }
}
impl<'a, T: 'a + Sized> Index<usize> for SliceMut<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}
impl<'a, T: 'a + Sized> IndexMut<usize> for SliceMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

pub struct SliceMutIter<'a, T> {
    iter: SliceIter<T>,
    _marker: PhantomData<&'a mut T>,
}
impl<'a, T> Iterator for SliceMutIter<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|mut ptr| unsafe { ptr.as_mut() })
    }
}
impl<'a, T> DoubleEndedIterator for SliceMutIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|mut ptr| unsafe { ptr.as_mut() })
    }
}
impl<'a, T> ExactSizeIterator for SliceMutIter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T> IntoIterator for SliceMut<'a, T> {
    type Item = &'a mut T;
    type IntoIter = SliceMutIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        SliceMutIter {
            iter: self.ptr.into_iter(),
            _marker: PhantomData,
        }
    }
}
