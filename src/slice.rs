use std::{marker::PhantomData, ops::Range, ptr::NonNull};

pub trait AsSlice<T: Sized> {
    /// required method
    unsafe fn slice_ptr(&self) -> SlicePtr<T>;
    /// provided methods
    fn len(&self) -> usize {
        unsafe { self.slice_ptr().len() }
    }
}
pub trait SliceRefExt<T>: AsSlice<T> {
    fn slice_ref<'a>(&'a self) -> SliceRef<'a, T> {
        unsafe { self.slice_ptr().as_ref() }
    }
    fn range_ref<'a>(&'a self, range: Range<usize>) -> SliceRef<'a, T> {
        unsafe { self.slice_ptr().range(range).as_ref() }
    }
    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {
        unsafe { self.slice_ptr().get_ref(index) }
    }
    fn iter<'a>(&'a self) -> SliceRefIter<'a, T> {
        self.slice_ref().into_iter()
    }
}
pub trait SliceMutExt<T>: AsSlice<T> + SliceRefExt<T> {
    fn slice_mut<'a>(&'a mut self) -> SliceMut<'a, T> {
        unsafe { self.slice_ptr().as_mut() }
    }
    fn range_mut<'a>(&'a mut self, range: Range<usize>) -> SliceMut<'a, T> {
        unsafe { self.slice_ptr().range(range).as_mut() }
    }
    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {
        unsafe { self.slice_ptr().get_mut(index) }
    }
    fn iter_mut<'a>(&'a mut self) -> SliceMutIter<'a, T> {
        self.slice_mut().into_iter()
    }
    fn reverse(&mut self) {
        unsafe { self.slice_ptr().reverse() }
    }
}

// impl<T: AsSlice> Index<usize> for T {
//     type Output = T::Target;
//     fn index(&self, index: usize) -> &Self::Output {
//         self.get(index).expect("index out of bounds")
//     }
// }
// impl<T: AsSlice> IndexMut<usize> for T {
//     fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//         self.get_mut(index).expect("index out of bounds")
//     }
// }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlicePtr<T> {
    pub head: NonNull<T>,
    pub len: usize,
}
impl<T> SlicePtr<T> {
    pub fn zst_slice() -> Self {
        Self {
            head: NonNull::dangling(),
            len: 0,
        }
    }
    pub unsafe fn new(head: NonNull<T>, len: usize) -> Self {
        Self { head, len }
    }
    pub unsafe fn range(self, range: Range<usize>) -> Self {
        assert!(range.start <= range.end && range.end <= self.len);
        if std::mem::size_of::<T>() == 0 {
            self
        } else {
            Self {
                head: unsafe { self.head.add(range.start) },
                len: range.end - range.start,
            }
        }
    }
    pub fn head(&self) -> NonNull<T> {
        self.head
    }
    pub fn len(self) -> usize {
        self.len
    }
    pub fn head_mut(&mut self) -> &mut NonNull<T> {
        &mut self.head
    }
    pub fn len_mut(&mut self) -> &mut usize {
        &mut self.len
    }
    pub fn get(self, index: usize) -> Option<NonNull<T>> {
        if index >= self.len {
            None
        } else {
            Some(unsafe { self.head.add(index) })
        }
    }
    pub unsafe fn get_ref<'a>(self, index: usize) -> Option<&'a T> {
        self.get(index).map(|ptr| unsafe { &*ptr.as_ptr() })
    }
    pub unsafe fn get_mut<'a>(self, index: usize) -> Option<&'a mut T> {
        self.get(index).map(|ptr| unsafe { &mut *ptr.as_ptr() })
    }
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
    pub fn split_at(self, index: usize) -> (Self, Self) {
        assert!(index <= self.len);
        let mid = unsafe { self.head.add(index) };
        (
            Self {
                head: self.head,
                len: index,
            },
            Self {
                head: mid,
                len: self.len - index,
            },
        )
    }
    pub fn reverse(self) {
        if self.len <= 1 {
            return;
        }
        let mut left = self.head;
        let mut right = unsafe { self.head.add(self.len - 1) };
        while left < right {
            unsafe {
                std::ptr::swap(left.as_ptr(), right.as_ptr());
                left = left.add(1);
                right = right.sub(1);
            }
        }
    }
    pub fn sort_by<F>(self, compare: F)
    where
        F: Fn(&T, &T) -> std::cmp::Ordering + Copy,
    {
        unimplemented!()
    }
    pub unsafe fn as_ref<'a>(self) -> SliceRef<'a, T> {
        SliceRef {
            ptr: self,
            _marker: PhantomData,
        }
    }
    pub unsafe fn as_mut<'a>(self) -> SliceMut<'a, T> {
        SliceMut {
            ptr: self,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for SlicePtr<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head,
            len: self.len,
        }
    }
}
impl<T> Copy for SlicePtr<T> {}

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
impl<T> IntoIterator for SlicePtr<T> {
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
pub struct SliceRef<'a, T> {
    ptr: SlicePtr<T>,
    _marker: PhantomData<&'a T>,
}
impl<'a, T> AsSlice<T> for SliceRef<'a, T> {
    unsafe fn slice_ptr(&self) -> SlicePtr<T> {
        self.ptr
    }
}
impl<'a, T> SliceRefExt<T> for SliceRef<'a, T> {}
impl<'a, T: 'a + Sized> Clone for SliceRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T: 'a + Sized> Copy for SliceRef<'a, T> {}

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
impl<'a, T> From<SliceIter<T>> for SliceRefIter<'a, T> {
    fn from(value: SliceIter<T>) -> Self {
        Self {
            iter: value,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliceMut<'a, T: 'a> {
    ptr: SlicePtr<T>,
    _marker: PhantomData<&'a mut T>,
}
impl<'a, T> AsSlice<T> for SliceMut<'a, T> {
    unsafe fn slice_ptr(&self) -> SlicePtr<T> {
        self.ptr
    }
}
impl<'a, T> SliceRefExt<T> for SliceMut<'a, T> {}
impl<'a, T> SliceMutExt<T> for SliceMut<'a, T> {}

// impl<'a, T: 'a + Sized> Clone for SliceMut<'a, T> {
//     fn clone(&self) -> Self {
//         Self {
//             ptr: self.ptr,
//             _marker: PhantomData,
//         }
//     }
// }
// impl<'a, T: 'a + Sized> Copy for SliceMut<'a, T> {}
// impl<'a, T: 'a + Sized> Deref for SliceMut<'a, T> {
//     type Target = SliceRef<'a, T>;
//     fn deref(&self) -> &Self::Target {
//         unsafe { &*(self as *const Self as *const SliceRef<T>) }
//     }
// }
// impl<'a, T: 'a + Sized> Index<usize> for SliceMut<'a, T> {
//     type Output = T;
//     fn index(&self, index: usize) -> &Self::Output {
//         self.get(index).expect("index out of bounds")
//     }
// }
// impl<'a, T: 'a + Sized> IndexMut<usize> for SliceMut<'a, T> {
//     fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//         self.get_mut(index).expect("index out of bounds")
//     }
// }

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
impl<'a, T> From<SliceIter<T>> for SliceMutIter<'a, T> {
    fn from(value: SliceIter<T>) -> Self {
        Self {
            iter: value,
            _marker: PhantomData,
        }
    }
}
