#![warn(clippy::manual_range_patterns)]

fn main() {
    let f = 6;
    let ch = '0';
    let byte = b'0';

    let _ = matches!(f, 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10);
    //~^ manual_range_patterns
    let _ = matches!(f, 4 | 2 | 3 | 1 | 5 | 6 | 9 | 7 | 8 | 10);
    //~^ manual_range_patterns
    let _ = matches!(f, 4 | 2 | 3 | 1 | 5 | 6 | 9 | 8 | 10); // 7 is missing
    let _ = matches!(f, | 4);
    let _ = matches!(f, 4 | 5);
    let _ = matches!(f, 1 | 2147483647);
    let _ = matches!(f, 0 | 2147483647);
    let _ = matches!(f, -2147483647 | 2147483647);
    let _ = matches!(f, 1 | (2..=4));
    //~^ manual_range_patterns
    let _ = matches!(f, 1 | (2..4));
    //~^ manual_range_patterns
    let _ = matches!(f, (1..=10) | (2..=13) | (14..=48324728) | 48324729);
    //~^ manual_range_patterns
    let _ = matches!(f, 0 | (1..=10) | 48324730 | (2..=13) | (14..=48324728) | 48324729);
    //~^ manual_range_patterns
    let _ = matches!(f, 0..=1 | 0..=2 | 0..=3);
    //~^ manual_range_patterns
    #[allow(clippy::match_like_matches_macro)]
    let _ = match f {
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 => true,
        //~^ manual_range_patterns
        _ => false,
    };
    let _ = matches!(f, -1 | -5 | 3 | -2 | -4 | -3 | 0 | 1 | 2);
    //~^ manual_range_patterns
    let _ = matches!(f, -1 | -5 | 3 | -2 | -4 | -3 | 0 | 1); // 2 is missing
    let _ = matches!(f, -1_000_000..=1_000_000 | -1_000_001 | 1_000_001);
    //~^ manual_range_patterns
    let _ = matches!(f, -1_000_000..=1_000_000 | -1_000_001 | 1_000_002);

    matches!(f, 0x00 | 0x01 | 0x02 | 0x03);
    //~^ manual_range_patterns
    matches!(f, 0x00..=0x05 | 0x06 | 0x07);
    //~^ manual_range_patterns
    matches!(f, -0x09 | -0x08 | -0x07..=0x00);
    //~^ manual_range_patterns
    matches!(ch, 'a' | 'b' | 'c' | 'd' | 'e');
    //~^ manual_range_patterns
    matches!(ch, '0' | '1' | '2' | '3' | '4');
    //~^ manual_range_patterns
    matches!(ch, '0' | '1' | '2' | '4'); // '3' is missing
    matches!(ch, '<' | '=' | '>'); // range is not intuitive
    matches!(ch, '/' | '0'..='9'); // range is not intuitive
    matches!(byte, b'0' | b'1' | b'2' | b'3');
    //~^ manual_range_patterns
    matches!(byte, b'<' | b'=' | b'>'); // range is not intuitive

    matches!(f, 0..5 | 5);
    //~^ manual_range_patterns
    matches!(f, 0 | 1..5);
    //~^ manual_range_patterns

    matches!(f, 0..=5 | 6..10);
    //~^ manual_range_patterns
    matches!(f, 0..5 | 5..=10);
    //~^ manual_range_patterns
    matches!(f, 5..=10 | 0..5);
    //~^ manual_range_patterns

    macro_rules! mac {
        ($e:expr) => {
            matches!($e, 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10)
            //~^ manual_range_patterns
        };
    }
    mac!(f);

    #[rustfmt::skip]
    let _ = match f {
        | 2..=15 => 4,
        | 241..=254 => 5,
        | 255 => 6,
        | _ => 7,
    };
}
