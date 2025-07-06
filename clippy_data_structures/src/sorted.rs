use crate::traits::{VecLike, VecLikeCapacity};
use arrayvec::ArrayVec;
use core::cmp::Ordering;

/// Determines whether a binary or linear search should be used when searching for `count` sorted
/// items in a sorted list of size `len`.
#[inline]
fn should_binary_search_gen(list_size: usize, search_count: usize) -> bool {
    // Using binary search has a complexity of `O(log2(len) * count)` with an average case only slightly
    // better. This roughly calculates if the binary search will be faster, erring on the side of a
    // linear search.

    // This is essentially `count < len / len.ilog2().next_power_of_two() / 2`, but with better codegen.
    let log2 = (usize::BITS - 1).wrapping_sub(list_size.leading_zeros());
    search_count < list_size.wrapping_shr(usize::BITS - log2.leading_zeros())
}

pub trait ShouldBinarySearchSpec {
    fn should_binary_search(list_size: usize, search_count: usize) -> bool;
}
impl<T: ?Sized> ShouldBinarySearchSpec for T {
    #[inline]
    default fn should_binary_search(list_size: usize, search_count: usize) -> bool {
        should_binary_search_gen(list_size, search_count)
    }
}
impl<T, const N: usize> ShouldBinarySearchSpec for ArrayVec<T, N> {
    #[inline]
    fn should_binary_search(list_size: usize, search_count: usize) -> bool {
        N > 6 && should_binary_search_gen(list_size, search_count)
    }
}
impl<T, const N: usize> ShouldBinarySearchSpec for crate::vec_set::VecSet<ArrayVec<T, N>> {
    #[inline]
    fn should_binary_search(list_size: usize, search_count: usize) -> bool {
        N > 6 && should_binary_search_gen(list_size, search_count)
    }
}

pub fn linear_search_by<T>(slice: &[T], mut f: impl FnMut(&T) -> Ordering) -> Result<usize, usize> {
    for (i, item) in slice.iter().enumerate() {
        match f(item) {
            Ordering::Less => {},
            Ordering::Equal => return Ok(i),
            Ordering::Greater => return Err(i),
        }
    }
    Err(slice.len())
}

pub fn fill_empty_from_iter_union<T: VecLike + Extend<T::Item>>(
    dst: &mut T,
    mut xs: impl Iterator<Item = T::Item>,
    mut ys: impl Iterator<Item = T::Item>,
    mut cmp: impl FnMut(&T::Item, &T::Item) -> Ordering,
) {
    let mut next_x = xs.next();
    let mut next_y = ys.next();
    loop {
        match (next_x, next_y) {
            (Some(x), Some(y)) => match cmp(&x, &y) {
                Ordering::Equal => {
                    dst.push(x);
                    next_x = xs.next();
                    next_y = ys.next();
                },
                Ordering::Less => {
                    dst.push(x);
                    next_x = xs.next();
                    next_y = Some(y);
                },
                Ordering::Greater => {
                    dst.push(y);
                    next_x = Some(x);
                    next_y = ys.next();
                },
            },
            (Some(x), None) => {
                dst.push(x);
                dst.extend(xs);
                break;
            },
            (None, Some(y)) => {
                dst.push(y);
                dst.extend(ys);
                break;
            },
            (None, None) => break,
        }
    }
}

/// Merges the contents of the iterator into the list.
///
/// Will panic with debug assertions enabled if the input sequence is not a sorted set.
fn union_impl<T>(
    list: &mut T,
    mut items: impl Iterator<Item = T::Item>,
    mut search: impl FnMut(&[T::Item], &T::Item) -> Result<usize, usize>,
    mut merge: impl FnMut(&mut T::Item, T::Item),
) where
    T: VecLike + Extend<T::Item> + ?Sized,
{
    let mut i = 0usize;
    while let Some(next) = items.next() {
        let slice = &mut list.borrow_mut()[i..];
        match search(slice, &next) {
            Ok(j) => {
                merge(&mut slice[j], next);
                i += j;
            },
            Err(j) if j != slice.len() => {
                list.insert(i + j, next);
                i += j;
            },
            Err(_) => {
                list.push(next);
                list.extend(items);
                return;
            },
        }
    }
}

/// Performs a union between two sorted sets, storing the result in the first.
///
/// Both lists must be sorted and contain no duplicates according to the given comparison function.
/// For any duplicates between the two lists the given merge function will be used to combine the
/// two values. This function must not change the sort order of the item.
pub fn union<T>(
    list: &mut T,
    items: impl IntoIterator<Item = T::Item>,
    mut cmp: impl FnMut(&T::Item, &T::Item) -> Ordering,
    merge: impl FnMut(&mut T::Item, T::Item),
) where
    T: VecLikeCapacity + Extend<T::Item> + ?Sized,
{
    let items = items.into_iter();
    let (min, max) = items.size_hint();
    list.reserve(min);
    let incoming = match max {
        Some(max) => min.midpoint(max),
        None => usize::MAX,
    };
    if <T as ShouldBinarySearchSpec>::should_binary_search(list.borrow().len(), incoming) {
        union_impl(list, items, |list, item| list.binary_search_by(|x| cmp(x, item)), merge);
    } else {
        union_impl(
            list,
            items,
            |list, item| linear_search_by(list, |x| cmp(x, item)),
            merge,
        );
    }
}

pub fn is_subset_of_linear<T: Ord>(xs: &[T], ys: &[T]) -> bool {
    let mut y = ys.iter();
    'outer: for x in xs {
        for y in &mut y {
            match x.cmp(y) {
                Ordering::Equal => continue 'outer,
                Ordering::Less => return false,
                Ordering::Greater => {},
            }
        }
        return false;
    }
    true
}

pub fn is_subset_of_binary<T: Ord>(xs: &[T], mut ys: &[T]) -> bool {
    for x in xs {
        match ys.binary_search(x) {
            Ok(i) => ys = &ys[i + 1..],
            Err(_) => return false,
        }
    }
    true
}

pub fn is_slice_set<T: Ord>(slice: &[T]) -> bool {
    slice.array_windows::<2>().all(|[x, y]| x.cmp(y).is_lt())
}
