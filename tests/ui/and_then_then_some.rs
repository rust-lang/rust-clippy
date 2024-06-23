#![warn(clippy::and_then_then_some)]

fn main() {
	let x = Some("foo".to_string());

    let _y = x.clone().and_then(|v| v.starts_with('f')
        .then_some(v));

	/*let _z = x.clone().and_then(|v| bool::then_some(v.starts_with('f'), v));
	// even if it's called as an associated method with a block body
	let _w = Option::and_then(x, |v: String| {
		bool::then_some(v.starts_with('f'), v)
	});*/
}
