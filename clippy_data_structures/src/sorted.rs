use core::cmp::Ordering;
use core::mem::MaybeUninit;

/// Determines whether a binary or linear search should be used when searching for `search_count`
/// sorted items in a sorted list of the given size.
#[inline]
pub fn should_binary_search(list_size: usize, search_count: usize) -> bool {
    // Using binary search has a complexity of `O(log2(list_size) * search_count)` with an average
    // case only slightly better. This roughly calculates if the binary search will be faster,
    // erring on the side of a linear search.

    // This is essentially `search_count < list_size / list_size.ilog2().next_power_of_two() / 2`,
    // but with better codegen.
    let log2 = (usize::BITS - 1).wrapping_sub(list_size.leading_zeros());
    // If `log2` is `MAX` then `list_size` is zero. Shifting by the maximum amount is fine.
    // If `log2` is 64 then `list_size` is one. Shifting by zero is fine.
    // In all other cases `log2` will be in the `0..BITS` range.
    search_count < list_size.wrapping_shr(usize::BITS - log2.leading_zeros())
}

/// Merges the two sorted lists into `dst`, discarding any duplicates between the two.
///
/// # Panics
/// Panics if `dst` is too small to contain the merged list.
pub fn union_fill_uninit<T>(
    dst: &mut [MaybeUninit<T>],
    mut xs: impl Iterator<Item = T>,
    mut ys: impl Iterator<Item = T>,
    mut cmp: impl FnMut(&T, &T) -> Ordering,
) -> &mut [T] {
    // n.b. `dst_iter` must be moved exactly once for each item written.
    let mut dst_iter = dst.iter_mut();
    let mut next_x = xs.next();
    let mut next_y = ys.next();
    loop {
        match (next_x, next_y) {
            (Some(x), Some(y)) => match cmp(&x, &y) {
                Ordering::Equal => {
                    dst_iter.next().unwrap().write(x);
                    next_x = xs.next();
                    next_y = ys.next();
                },
                Ordering::Less => {
                    dst_iter.next().unwrap().write(x);
                    next_x = xs.next();
                    next_y = Some(y);
                },
                Ordering::Greater => {
                    dst_iter.next().unwrap().write(y);
                    next_x = Some(x);
                    next_y = ys.next();
                },
            },
            (Some(x), None) => {
                dst_iter.next().unwrap().write(x);
                xs.for_each(|x| {
                    dst_iter.next().unwrap().write(x);
                });
                break;
            },
            (None, Some(y)) => {
                dst_iter.next().unwrap().write(y);
                ys.for_each(|y| {
                    dst_iter.next().unwrap().write(y);
                });
                break;
            },
            (None, None) => break,
        }
    }

    let remain = dst_iter.into_slice().len();
    let end = dst.len() - remain;
    // Safety: Every item returned by `dst_iter` was written to.
    unsafe { dst[..end].assume_init_mut() }
}

pub fn is_subset_of_linear<T: Ord>(xs: &[T], ys: &[T]) -> bool {
    let mut ys = ys.iter();
    'outer: for x in xs {
        for y in &mut ys {
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

/// Checks is a slice is ordered with no duplicates.
pub fn is_slice_set<T: Ord>(slice: &[T]) -> bool {
    slice.array_windows::<2>().all(|[x, y]| x.cmp(y).is_lt())
}
