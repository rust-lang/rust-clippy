#![feature(associated_type_defaults)]
#![warn(clippy::type_complexity)]

type Alias = Vec<Vec<Box<(u32, u32, u32, u32)>>>; // no warning here

const CST: (u32, (u32, (u32, (u32, u32)))) = (0, (0, (0, (0, 0))));
//~^ type_complexity

static ST: (u32, (u32, (u32, (u32, u32)))) = (0, (0, (0, (0, 0))));
//~^ type_complexity

struct S {
    f: Vec<Vec<Box<(u32, u32, u32, u32)>>>,
    //~^ type_complexity
}

struct Ts(Vec<Vec<Box<(u32, u32, u32, u32)>>>);
//~^ type_complexity

enum E {
    Tuple(Vec<Vec<Box<(u32, u32, u32, u32)>>>),
    //~^ type_complexity
    Struct { f: Vec<Vec<Box<(u32, u32, u32, u32)>>> },
    //~^ type_complexity
}

impl S {
    const A: (u32, (u32, (u32, (u32, u32)))) = (0, (0, (0, (0, 0))));
    //~^ type_complexity

    fn impl_method(&self, p: Vec<Vec<Box<(u32, u32, u32, u32)>>>) {}
    //~^ type_complexity
}

trait T {
    const A: Vec<Vec<Box<(u32, u32, u32, u32)>>>;
    //~^ type_complexity

    type B = Vec<Vec<Box<(u32, u32, u32, u32)>>>;
    //~^ type_complexity

    fn method(&self, p: Vec<Vec<Box<(u32, u32, u32, u32)>>>);
    //~^ type_complexity

    fn def_method(&self, p: Vec<Vec<Box<(u32, u32, u32, u32)>>>) {}
    //~^ type_complexity
}

// Should not warn since there is likely no way to simplify this (#1013)
impl T for () {
    const A: Vec<Vec<Box<(u32, u32, u32, u32)>>> = vec![];

    type B = Vec<Vec<Box<(u32, u32, u32, u32)>>>;

    fn method(&self, p: Vec<Vec<Box<(u32, u32, u32, u32)>>>) {}
}

fn test1() -> Vec<Vec<Box<(u32, u32, u32, u32)>>> {
    //~^ type_complexity

    vec![]
}

fn test2(_x: Vec<Vec<Box<(u32, u32, u32, u32)>>>) {}
//~^ type_complexity

fn test3() {
    let _y: Vec<Vec<Box<(u32, u32, u32, u32)>>> = vec![];
    //~^ type_complexity
}

#[repr(C)]
struct D {
    // should not warn, since we don't have control over the signature (#3222)
    test4: extern "C" fn(
        itself: &D,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
        f: usize,
        g: usize,
        h: usize,
        i: usize,
    ),
}

struct MarkerBounds;
trait MarkerBound<T> {}
impl<T> MarkerBound<T> for MarkerBounds {}

// Should not warn, because no individual type argument inside the opaque bounds is complex.
fn many_simple_opaque_bound_type_arguments() -> impl MarkerBound<[u8; 0]>
+ MarkerBound<[u8; 1]>
+ MarkerBound<[u8; 2]>
+ MarkerBound<[u8; 3]>
+ MarkerBound<[u8; 4]>
+ MarkerBound<[u8; 5]>
+ MarkerBound<[u8; 6]>
+ MarkerBound<[u8; 7]>
+ MarkerBound<[u8; 8]> {
    MarkerBounds
}

// Should not warn, because factoring `impl Trait` into a type alias is not stable (#17195).
fn issue_17195<I, J>(
    left: I,
    right: J,
) -> std::iter::Map<std::iter::Zip<I::IntoIter, J::IntoIter>, impl FnMut((I::Item, I::Item)) -> [I::Item; 2]>
where
    I: IntoIterator,
    J: IntoIterator<Item = I::Item>,
{
    left.into_iter().zip(right).map(<[I::Item; 2]>::from)
}

// Complexity inside an opaque bound can still be factored out on stable.
fn complex_aliasable_opaque_bound() -> impl Fn(Vec<Vec<Box<(u32, u32, u32, u32)>>>) {
    //~^ type_complexity
    |_| {}
}

// The presence of an opaque type must not hide complexity in a sibling type that can be factored
// out.
fn complex_after_opaque() -> (impl Iterator<Item = u32>, Vec<Vec<Box<(u32, u32, u32, u32)>>>) {
    //~^ type_complexity
    (std::iter::empty(), vec![])
}

fn main() {}
