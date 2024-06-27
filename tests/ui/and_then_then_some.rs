#![warn(clippy::and_then_then_some)]

fn main() {
    let x = Some("foo".to_string());

    let _y = x.clone().and_then(|v| v.starts_with('f').then_some(v));

    let _z = x.clone().and_then(|v| bool::then_some(v.starts_with('f'), v));
    let _w = Option::and_then(x.clone(), |v: String| bool::then_some(v.starts_with('f'), v));
	#[allow(clippy::needless_return)]
	let _v = x.clone().and_then(|v| return v.starts_with('g').then_some(v));
}
