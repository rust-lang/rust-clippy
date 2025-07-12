use crate::sorted_set::merge_sorted;
use crate::{AsSlice, DerefSlice, VecBase};
use core::borrow::{Borrow, BorrowMut};
use core::mem::{self, transmute};
use core::ops::Deref;

#[derive(Default, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct VecMap<T: ?Sized> {
    data: T,
}
impl<T> VecMap<T>
where
    T: Default,
{
    #[inline]
    pub fn new() -> Self {
        Self { data: T::default() }
    }
}
impl<T, K, V> VecMap<T>
where
    T: ?Sized + AsSlice<Item = (K, V)>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.data.borrow().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.borrow().len() == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &VecMap<[(K, V)]> {
        // SAFETY: `Sorted<T>`` is a transparent wrapper around `T`.
        unsafe { transmute::<&[(K, V)], &VecMap<[(K, V)]>>(self.data.borrow()) }
    }

    #[inline]
    fn find<Q>(&self, item: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if T::MAX <= 6 {
            // Optimization for small `ArrayVec`s.
            self.data.borrow().binary_search_by(|x| x.0.borrow().cmp(item))
        } else {
            crate::sorted_set::linear_search_by_sorted(self.data.borrow(), |x| x.0.borrow().cmp(item))
        }
    }

    #[inline]
    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).is_ok()
    }

    #[inline]
    pub fn get<'a, Q>(&'a self, key: &Q) -> Option<&'a V>
    where
        K: 'a + Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| &self.data.borrow()[i].1)
    }
}
impl<T, K, V> VecMap<T>
where
    T: ?Sized + AsSlice<Item = (K, V)> + AsMut<[(K, V)]>,
{
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut VecMap<[(K, V)]> {
        // SAFETY: `Sorted<T>`` is a transparent wrapper around `T`.
        unsafe { transmute::<&mut [(K, V)], &mut VecMap<[(K, V)]>>(self.data.as_mut()) }
    }

    #[inline]
    pub fn get_mut<'a, Q>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where
        K: 'a + Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| &mut self.data.as_mut()[i].1)
    }
}
impl<T, K, V> VecMap<T>
where
    T: VecBase<Item = (K, V)>,
{
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.find(key) {
            Ok(i) => Some(self.data.remove(i).1),
            Err(_) => None,
        }
    }

    #[inline]
    pub fn retain(&mut self, f: impl FnMut(&mut T::Item) -> bool) {
        self.data.retain(f);
    }
}
impl<T, K, V> VecMap<T>
where
    T: VecBase<Item = (K, V)> + BorrowMut<[(K, V)]>,
    K: Ord,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.find(&key) {
            Ok(i) => Some(mem::replace(&mut self.data.borrow_mut()[i].1, value)),
            Err(i) => {
                self.data.insert(i, (key, value));
                None
            },
        }
    }
}
impl<T, K, V> VecMap<T>
where
    T: VecBase<Item = (K, V)> + FromIterator<(K, V)>,
    K: Ord,
{
    #[inline]
    pub fn from_sorted(items: impl IntoIterator<Item = (K, V)>) -> Self {
        Self {
            data: T::from_iter(items),
        }
    }
}
impl<T, K, V> VecMap<T>
where
    T: VecBase<Item = (K, V)> + Extend<(K, V)> + BorrowMut<[(K, V)]>,
    K: Ord,
{
    #[inline]
    pub fn insert_sorted(&mut self, items: impl IntoIterator<Item = T::Item>) {
        merge_sorted(&mut self.data, items, |x, y| x.0.cmp(&y.0), |x, y| x.1 = y.1);
    }

    #[inline]
    pub fn merge_sorted(&mut self, items: impl IntoIterator<Item = T::Item>, mut merge: impl FnMut(&mut V, V)) {
        merge_sorted(&mut self.data, items, |x, y| x.0.cmp(&y.0), |x, y| merge(&mut x.1, y.1));
    }
}

impl<T, K, V> Deref for VecMap<T>
where
    T: AsSlice<Item = (K, V)> + DerefSlice + ?Sized,
{
    type Target = VecMap<[(K, V)]>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, K, V> Borrow<VecMap<[(K, V)]>> for VecMap<T>
where
    T: AsSlice<Item = (K, V)> + DerefSlice + ?Sized,
{
    #[inline]
    fn borrow(&self) -> &VecMap<[(K, V)]> {
        self.as_slice()
    }
}

impl<T> IntoIterator for VecMap<T>
where
    T: IntoIterator,
{
    type Item = T::Item;
    type IntoIter = T::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
impl<'a, T> IntoIterator for &'a VecMap<T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.data).into_iter()
    }
}
impl<'a, T> IntoIterator for &'a mut VecMap<T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.data).into_iter()
    }
}
