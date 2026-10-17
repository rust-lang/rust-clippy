#![feature(type_alias_impl_trait)]
#![warn(clippy::type_complexity)]

struct MarkerBounds;
trait MarkerBound<T> {}
impl<T> MarkerBound<T> for MarkerBounds {}

fn many_simple_opaque_bound_type_arguments() -> impl MarkerBound<[u8; 0]>
//~^ type_complexity
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

fn main() {}
