//Wrap everything in macro to allow constant functions

#[macro_export]
macro_rules! specialized_panic_oob {
    ($method_name:expr, $index:expr, $len:expr) => {
        panic!(concat!("Simple fixed length array::", $method_name, ": index {} is out of bounds in vector of length {}"),
               $index, $len)
    }
}

pub trait FixedArray<T: Sized>: Sized {
    fn get_length(&self) -> usize;
    unsafe fn set_length(&mut self, new_len: usize);
    fn get_ptr(&self) -> *const T;
    fn get_mut_ptr(&mut self) -> *mut T;
}

// impl<'a, T: Sized + 'a> FixedArray<T> for &'a mut FixedArray<T> {
//     fn len(self) -> usize { self.len() }
//     fn set_len(&mut self, new_len: usize) -> { self.set_len(new_len)}
//     fn as_ptr(&self) -> *const $over_type { &*self.as_ptr() }
//     fn as_mut_ptr(&mut self) -> *mut $over_type { self.as_mut_ptr() }
// }

#[macro_export]
macro_rules! const_array_impl {
    ($array_name: ident, $over_type:ty, $capacity: expr ) => (
        pub struct $array_name {
            xs: core::mem::MaybeUninit<[$over_type; $capacity]>,
            len: usize,
        }

        impl FixedArray<$over_type> for $array_name {
            fn get_length(&self) -> usize { self.len() }
            unsafe fn set_length(&mut self, new_len: usize) { self.set_len(new_len) }
            fn get_ptr(&self) -> *const $over_type { self.as_ptr() }
            fn get_mut_ptr(&mut self) -> *mut $over_type { self.as_mut_ptr() }
        }

        impl FixedArray<$over_type> for &mut $array_name {
            fn get_length(&self) -> usize { self.len() }
            unsafe fn set_length(&mut self, new_len: usize) { self.set_len(new_len)}
            fn get_ptr(&self) -> *const $over_type { self.as_ptr() }
            fn get_mut_ptr(&mut self) -> *mut $over_type { self.as_mut_ptr() }
        }

        // impl<'a> FixedArray<$over_type> for &'a mut $array_name {
        //     fn len(&self) -> usize { self.len() }
        //     fn set_len(&mut self, new_len: usize) -> { self.set_len(new_len)}
        //     fn as_ptr(&self) -> *const $over_type { self.as_ptr() }
        //     fn as_mut_ptr(&mut self) -> *mut $over_type { self.as_mut_ptr() }
        // }


        impl $array_name {
            pub const CAPACITY: usize = $capacity;

            #[inline]
            pub const fn new() -> Self {
                unsafe {
                    Self { xs: core::mem::MaybeUninit::<[$over_type; $capacity]>::uninit(), len: 0 }
                }
            }

            #[inline]
            pub const fn len(&self) -> usize { self.len }

            #[inline]
            pub const fn is_empty(&self) -> bool { self.len() == 0 }

            #[inline(always)]
            pub const fn capacity(&self) -> usize { Self::CAPACITY }

            pub const fn is_full(&self) -> bool { self.len() == self.capacity() }

            pub const fn remaining_capacity(&self) -> usize {
                self.capacity() - self.len()
            }

            pub fn push(&mut self, element: $over_type) {
                self.try_push(element).unwrap()
            }

            pub fn try_push(&mut self, element: $over_type) -> Result<(), CapacityError<$over_type>> {
                if self.len() < Self::CAPACITY {
                    unsafe {
                        self.push_unchecked(element);
                    }
                    Ok(())
                } else {
                    Err(CapacityError::new(element))
                }
            }

            pub unsafe fn push_unchecked(&mut self, element: $over_type) {
                let len = self.len();
                debug_assert!(len < Self::CAPACITY);
                core::ptr::write(self.get_unchecked_ptr(len), element);
                self.set_len(len + 1);
            }

            unsafe fn get_unchecked_ptr(&mut self, index: usize) -> *mut $over_type {
                (self.xs.as_mut_ptr() as *mut $over_type).add(index)
            }

            pub fn insert(&mut self, index: usize, element: $over_type) {
                self.try_insert(index, element).unwrap()
            }

            pub fn try_insert(&mut self, index: usize, element: $over_type) -> Result<(), CapacityError<$over_type>> {
                if index > self.len() {
                    specialized_panic_oob!("try_insert", index, self.len())
                }
                if self.len() == self.capacity() {
                    return Err(CapacityError::new(element));
                }
                let len = self.len();

                // follows is just like Vec<T>
                unsafe { // infallible
                    // The spot to put the new value
                    {
                        let p: *mut _ = self.get_unchecked_ptr(index);
                        // Shift everything over to make space. (Duplicating the
                        // `index`th element into two consecutive places.)
                        core::ptr::copy(p, p.offset(1), len - index);
                        // Write it in, overwriting the first copy of the `index`th
                        // element.
                        core::ptr::write(p, element);
                    }
                    self.set_len(len + 1);
                }
                Ok(())
            }

            pub fn pop(&mut self) -> Option<$over_type> {
                if self.len() == 0 {
                    return None;
                }
                unsafe {
                    let new_len = self.len() - 1;
                    self.set_len(new_len);
                    Some(core::ptr::read(self.get_unchecked_ptr(new_len)))
                }
            }

            pub fn swap_remove(&mut self, index: usize) -> $over_type {
                self.swap_pop(index)
                    .unwrap_or_else(|| {
                        specialized_panic_oob!("swap_remove", index, self.len())
                    })
            }

            pub fn swap_pop(&mut self, index: usize) -> Option<$over_type> {
                let len = self.len();
                if index >= len {
                    return None;
                }
                self.swap(index, len - 1);
                self.pop()
            }

            pub fn remove(&mut self, index: usize) -> $over_type {
                self.pop_at(index)
                    .unwrap_or_else(|| {
                        specialized_panic_oob!("remove", index, self.len())
                    })
            }

            pub fn pop_at(&mut self, index: usize) -> Option<$over_type> {
                if index >= self.len() {
                    None
                } else {
                    self.drain(index..index + 1).next()
                }
            }

            pub fn truncate(&mut self, new_len: usize) {
                unsafe {
                    if new_len < self.len() {
                        let tail: *mut [_] = &mut self[new_len..];
                        self.len = new_len;
                        core::ptr::drop_in_place(tail);
                    }
                }
            }

            pub fn clear(&mut self) {
                self.truncate(0)
            }

            pub fn retain<F>(&mut self, mut f: F)
                where F: FnMut(&mut $over_type) -> bool
            {
                let len = self.len();
                let mut del = 0;
                {
                    let v = &mut **self;

                    for i in 0..len {
                        if !f(&mut v[i]) {
                            del += 1;
                        } else if del > 0 {
                            v.swap(i - del, i);
                        }
                    }
                }
                if del > 0 {
                    self.drain(len - del..);
                }
            }

            pub unsafe fn set_len(&mut self, length: usize) {
                debug_assert!(length <= self.capacity());
                self.len = length;
            }

            pub fn try_extend_from_slice(&mut self, other: &[$over_type]) -> Result<(), CapacityError>
                // where $over_type: Copy,
            {
                if self.remaining_capacity() < other.len() {
                    return Err(CapacityError::new(()));
                }

                let self_len = self.len();
                let other_len = other.len();

                unsafe {
                    let dst = (self.xs.as_mut_ptr() as *mut $over_type).offset(self_len as isize);
                    core::ptr::copy_nonoverlapping(other.as_ptr(), dst, other_len);
                    self.set_len(self_len + other_len);
                }
                Ok(())
            }

            pub fn drain<R>(&mut self, range: R) -> $crate::SimpleDrain<$array_name, $over_type>
                where R: core::ops::RangeBounds<usize>
            {
                use core::ops::Bound;
                // Memory safety
                //
                // When the SimpleDrain is first created, it shortens the length of
                // the source vector to make sure no uninitialized or moved-from elements
                // are accessible at all if the SimpleDrain's destructor never gets to run.
                //
                // SimpleDrain will ptr::read out the values to remove.
                // When finished, remaining tail of the vec is copied back to cover
                // the hole, and the vector length is restored to the new length.
                //
                let len = self.len();
                let start = match range.start_bound() {
                    Bound::Unbounded => 0,
                    Bound::Included(&i) => i,
                    Bound::Excluded(&i) => i.saturating_add(1),
                };
                let end = match range.end_bound() {
                    Bound::Excluded(&j) => j,
                    Bound::Included(&j) => j.saturating_add(1),
                    Bound::Unbounded => len,
                };
                self.drain_range(start, end)
            }

            fn drain_range(&mut self, start: usize, end: usize) -> $crate::SimpleDrain<$array_name, $over_type>
            {
                let len = self.len();

                // bounds check happens here (before length is changed!)
                let range_slice: *const _ = &self[start..end];

                // Calling `set_len` creates a fresh and thus unique mutable references, making all
                // older aliases we created invalid. So we cannot call that function.
                self.len = start;

                unsafe {
                    $crate::SimpleDrain::<$array_name, $over_type> {
                        tail_start: end,
                        tail_len: len - end,
                        iter: (*range_slice).iter(),
                        vec: self as *mut _,
                    }
                }
            }

            pub fn into_inner(self) -> Result<[$over_type; Self::CAPACITY], Self> {
                if self.len() < self.capacity() {
                    Err(self)
                } else {
                    unsafe {
                        let array = core::ptr::read(self.xs.as_ptr());
                        mem::forget(self);
                        Ok(array)
                    }
                }
            }

            pub fn as_slice(&self) -> &[$over_type] {
                self
            }

            pub fn as_mut_slice(&mut self) -> &mut [$over_type] {
                self
            }

            pub fn as_ptr(&self) -> *const $over_type {
                self.xs.as_ptr() as *const $over_type
            }

            pub fn as_mut_ptr(&mut self) -> *mut $over_type {
                self.xs.as_mut_ptr() as *mut $over_type
            }
        }

        impl core::ops::Drop for $array_name {
            fn drop(&mut self) {
                self.clear();
        
                // MaybeUninit inhibits array's drop
            }
        }

        impl core::ops::Deref for $array_name {
            type Target = [$over_type];
            #[inline]
            fn deref(&self) -> &[$over_type] {
                unsafe {
                    core::slice::from_raw_parts(self.xs.as_ptr() as *const $over_type, self.len())
                }
            }
        }
        
        impl core::ops::DerefMut for $array_name {
            #[inline]
            fn deref_mut(&mut self) -> &mut [$over_type] {
                let len = self.len();
                unsafe {
                    core::slice::from_raw_parts_mut(self.xs.as_mut_ptr() as *mut $over_type, len)
                }
            }
        }

        impl core::borrow::Borrow<[$over_type]> for $array_name {
            fn borrow(&self) -> &[$over_type] { self }
        }

        impl core::borrow::BorrowMut<[$over_type]> for $array_name {
            fn borrow_mut(&mut self) -> &mut [$over_type] { self }
        }

        impl AsRef<[$over_type]> for $array_name {
            fn as_ref(&self) -> &[$over_type] { self }
        }

        impl AsMut<[$over_type]> for $array_name {
            fn as_mut(&mut self) -> &mut [$over_type] { self }
        }

        // impl<A: Array> fmt::Debug for ArrayVec<A> where A::Item: fmt::Debug {
        //     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (**self).fmt(f) }
        // }

        impl core::default::Default for $array_name {
            /// Return an empty array
            fn default() -> $array_name {
                $array_name::new()
            }
        }

        impl From<[$over_type; $capacity]> for $array_name {
            fn from(array: [$over_type; $capacity]) -> Self {
                Self { xs: core::mem::MaybeUninit::new(array), len: Self::CAPACITY }
            }
        }

        impl<'a> IntoIterator for &'a $array_name {
            type Item = &'a $over_type;
            type IntoIter = core::slice::Iter<'a, $over_type>;
            fn into_iter(self) -> Self::IntoIter { self.iter() }
        }

        impl<'a> IntoIterator for &'a mut $array_name {
            type Item = &'a mut $over_type;
            type IntoIter = core::slice::IterMut<'a, $over_type>;
            fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
        }

        impl Extend<$over_type> for $array_name {
            fn extend<T: IntoIterator<Item=$over_type>>(&mut self, iter: T) {
                let take = self.capacity() - self.len();
                unsafe {
                    let len = self.len();
                    let mut ptr = Self::raw_ptr_add(self.as_mut_ptr(), len);
                    let end_ptr = Self::raw_ptr_add(ptr, take);
                    // Keep the length in a separate variable, write it back on scope
                    // exit. To help the compiler with alias analysis and stuff.
                    // We update the length to handle panic in the iteration of the
                    // user's iterator, without dropping any elements on the floor.
                    let mut guard = len;
                    let mut iter = iter.into_iter();
                    loop {
                        if ptr == end_ptr { break; }
                        if let Some(elt) = iter.next() {
                            Self::raw_ptr_write(ptr, elt);
                            ptr = Self::raw_ptr_add(ptr, 1);
                            guard += 1;
                        } else {
                            break;
                        }
                    }

                    self.len = guard;
                }
            }
        }

        impl $array_name {
            /// Rawptr add but uses arithmetic distance for ZST
            unsafe fn raw_ptr_add<T>(ptr: *mut T, offset: usize) -> *mut T {
                if core::mem::size_of::<T>() == 0 {
                    // Special case for ZST
                    (ptr as usize).wrapping_add(offset) as _
                } else {
                    ptr.offset(offset as isize)
                }
            }

            unsafe fn raw_ptr_write<T>(ptr: *mut T, value: T) {
                if core::mem::size_of::<T>() == 0 {
                    /* nothing */
                } else {
                    core::ptr::write(ptr, value)
                }
            }
        }

        impl core::iter::FromIterator<$over_type> for $array_name {
            fn from_iter<T: IntoIterator<Item=$over_type>>(iter: T) -> Self {
                let mut array = $array_name::new();
                array.extend(iter);
                array
            }
        }
    )
}


// /// Iterate the `ArrayVec` with each element by value.
// ///
// /// The vector is consumed by this operation.
// ///
// /// ```
// /// use arrayvec::ArrayVec;
// ///
// /// for elt in ArrayVec::from([1, 2, 3]) {
// ///     // ...
// /// }
// /// ```
// impl<A: Array> IntoIterator for ArrayVec<A> {
//     type Item = A::Item;
//     type IntoIter = IntoIter<A>;
//     fn into_iter(self) -> IntoIter<A> {
//         IntoIter { index: Index::from(0), v: self, }
//     }
// }


// /// By-value iterator for `ArrayVec`.
// pub struct IntoIter<A: Array> {
//     index: A::Index,
//     v: ArrayVec<A>,
// }

// impl<A: Array> Iterator for IntoIter<A> {
//     type Item = A::Item;

//     fn next(&mut self) -> Option<A::Item> {
//         if self.index == self.v.len {
//             None
//         } else {
//             unsafe {
//                 let index = self.index.to_usize();
//                 self.index = Index::from(index + 1);
//                 Some(ptr::read(self.v.get_unchecked_ptr(index)))
//             }
//         }
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         let len = self.v.len() - self.index.to_usize();
//         (len, Some(len))
//     }
// }

// impl<A: Array> DoubleEndedIterator for IntoIter<A> {
//     fn next_back(&mut self) -> Option<A::Item> {
//         if self.index == self.v.len {
//             None
//         } else {
//             unsafe {
//                 let new_len = self.v.len() - 1;
//                 self.v.set_len(new_len);
//                 Some(ptr::read(self.v.get_unchecked_ptr(new_len)))
//             }
//         }
//     }
// }

// impl<A: Array> ExactSizeIterator for IntoIter<A> { }

// impl<A: Array> Drop for IntoIter<A> {
//     fn drop(&mut self) {
//         // panic safety: Set length to 0 before dropping elements.
//         let index = self.index.to_usize();
//         let len = self.v.len();
//         unsafe {
//             self.v.set_len(0);
//             let elements = slice::from_raw_parts_mut(
//                 self.v.get_unchecked_ptr(index),
//                 len - index);
//             ptr::drop_in_place(elements);
//         }
//     }
// }

// impl<A: Array> Clone for IntoIter<A>
// where
//     A::Item: Clone,
// {
//     fn clone(&self) -> IntoIter<A> {
//         self.v[self.index.to_usize()..]
//             .iter()
//             .cloned()
//             .collect::<ArrayVec<A>>()
//             .into_iter()
//     }
// }

// impl<A: Array> fmt::Debug for IntoIter<A>
// where
//     A::Item: fmt::Debug,
// {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_list()
//             .entries(&self.v[self.index.to_usize()..])
//             .finish()
//     }
// }

/// A draining iterator for `ArrayVec`.
pub struct SimpleDrain<'a, A, T> 
    where A: FixedArray<T>,
        T: Sized
{
    /// Index of tail to preserve
    pub tail_start: usize,
    /// Length of tail
    pub tail_len: usize,
    /// Current remaining range to remove
    pub iter: core::slice::Iter<'a, T>,
    pub vec: *mut A,
}

unsafe impl<'a, T: Sync, A: FixedArray<T> + Sync> Sync for SimpleDrain<'a, A, T> {}
unsafe impl<'a, T: Send, A: FixedArray<T> + Send> Send for SimpleDrain<'a, A, T> {}

impl<'a, T: Sized + 'a, A: FixedArray<T>> Iterator for SimpleDrain<'a, A, T>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|elt|
            unsafe {
                core::ptr::read(elt as *const _)
            }
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: Sized + 'a, A: FixedArray<T>> DoubleEndedIterator for SimpleDrain<'a, A, T>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|elt|
            unsafe {
                core::ptr::read(elt as *const _)
            }
        )
    }
}

impl<'a, T: Sized + 'a, A: FixedArray<T>> ExactSizeIterator for SimpleDrain<'a, A, T> {}

impl<'a, T: Sized + 'a, A: FixedArray<T>> Drop for SimpleDrain<'a, A, T> 
{
    fn drop(&mut self) {
        // len is currently 0 so panicking while dropping will not cause a double drop.

        // exhaust self first
        while let Some(_) = self.next() { }

        if self.tail_len > 0 {
            unsafe {
                let source_vec = &mut *self.vec;
                // memmove back untouched tail, update to new length
                let start = source_vec.get_length();
                let tail = self.tail_start;
                let src = source_vec.get_ptr().offset(tail as isize);
                let dst = source_vec.get_mut_ptr().offset(start as isize);
                core::ptr::copy(src, dst, self.tail_len);
                source_vec.set_length(start + self.tail_len);
            }
        }
    }
}

// struct ScopeExitGuard<T, Data, F>
//     where F: FnMut(&Data, &mut T)
// {
//     value: T,
//     data: Data,
//     f: F,
// }

// impl<T, Data, F> Drop for ScopeExitGuard<T, Data, F>
//     where F: FnMut(&Data, &mut T)
// {
//     fn drop(&mut self) {
//         (self.f)(&self.data, &mut self.value)
//     }
// }




// impl<A: Array> Clone for ArrayVec<A>
//     where A::Item: Clone
// {
//     fn clone(&self) -> Self {
//         self.iter().cloned().collect()
//     }

//     fn clone_from(&mut self, rhs: &Self) {
//         // recursive case for the common prefix
//         let prefix = cmp::min(self.len(), rhs.len());
//         self[..prefix].clone_from_slice(&rhs[..prefix]);

//         if prefix < self.len() {
//             // rhs was shorter
//             for _ in 0..self.len() - prefix {
//                 self.pop();
//             }
//         } else {
//             let rhs_elems = rhs[self.len()..].iter().cloned();
//             self.extend(rhs_elems);
//         }
//     }
// }

// impl<A: Array> Hash for ArrayVec<A>
//     where A::Item: Hash
// {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         Hash::hash(&**self, state)
//     }
// }

// impl<A: Array> PartialEq for ArrayVec<A>
//     where A::Item: PartialEq
// {
//     fn eq(&self, other: &Self) -> bool {
//         **self == **other
//     }
// }

// impl<A: Array> PartialEq<[A::Item]> for ArrayVec<A>
//     where A::Item: PartialEq
// {
//     fn eq(&self, other: &[A::Item]) -> bool {
//         **self == *other
//     }
// }

// impl<A: Array> Eq for ArrayVec<A> where A::Item: Eq { }

// impl<A: Sized> PartialOrd for ArrayVec<A> where A::Item: PartialOrd {
//     fn partial_cmp(&self, other: &ArrayVec<A>) -> Option<cmp::Ordering> {
//         (**self).partial_cmp(other)
//     }

//     fn lt(&self, other: &Self) -> bool {
//         (**self).lt(other)
//     }

//     fn le(&self, other: &Self) -> bool {
//         (**self).le(other)
//     }

//     fn ge(&self, other: &Self) -> bool {
//         (**self).ge(other)
//     }

//     fn gt(&self, other: &Self) -> bool {
//         (**self).gt(other)
//     }
// }

// impl<A: Array> Ord for ArrayVec<A> where A::Item: Ord {
//     fn cmp(&self, other: &ArrayVec<A>) -> cmp::Ordering {
//         (**self).cmp(other)
//     }
// }

// #[cfg(feature="std")]
// /// `Write` appends written data to the end of the vector.
// ///
// /// Requires `features="std"`.
// impl<A: Array<Item=u8>> io::Write for ArrayVec<A> {
//     fn write(&mut self, data: &[u8]) -> io::Result<usize> {
//         let len = cmp::min(self.remaining_capacity(), data.len());
//         let _result = self.try_extend_from_slice(&data[..len]);
//         debug_assert!(_result.is_ok());
//         Ok(len)
//     }
//     fn flush(&mut self) -> io::Result<()> { Ok(()) }
// }

// #[cfg(feature="serde")]
// /// Requires crate feature `"serde"`
// impl<T: Serialize, A: Array<Item=T>> Serialize for ArrayVec<A> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where S: Serializer
//     {
//         serializer.collect_seq(self)
//     }
// }

// #[cfg(feature="serde")]
// /// Requires crate feature `"serde"`
// impl<'de, T: Deserialize<'de>, A: Array<Item=T>> Deserialize<'de> for ArrayVec<A> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where D: Deserializer<'de>
//     {
//         use serde::de::{Visitor, SeqAccess, Error};
//         use std::marker::PhantomData;

//         struct ArrayVecVisitor<'de, T: Deserialize<'de>, A: Array<Item=T>>(PhantomData<(&'de (), T, A)>);

//         impl<'de, T: Deserialize<'de>, A: Array<Item=T>> Visitor<'de> for ArrayVecVisitor<'de, T, A> {
//             type Value = ArrayVec<A>;

//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 write!(formatter, "an array with no more than {} items", A::CAPACITY)
//             }

//             fn visit_seq<SA>(self, mut seq: SA) -> Result<Self::Value, SA::Error>
//                 where SA: SeqAccess<'de>,
//             {
//                 let mut values = ArrayVec::<A>::new();

//                 while let Some(value) = seq.next_element()? {
//                     if let Err(_) = values.try_push(value) {
//                         return Err(SA::Error::invalid_length(A::CAPACITY + 1, &self));
//                     }
//                 }

//                 Ok(values)
//             }
//         }

//         deserializer.deserialize_seq(ArrayVecVisitor::<T, A>(PhantomData))
//     }
// }
