#![feature(rustc_private)]
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)]

use clippy_data_structures::BitSlice2d;
use clippy_data_structures::bit_slice::WORD_BITS;

#[test]
#[rustfmt::skip]
fn row_iter_copy_move_3x3() {
    let mut x = [
        0, 0, 0,
        0b1010, 0b1010, 0b1010,
        0b0101, 0b0101, 0b0101,
    ];
    let mut x = BitSlice2d::<'_, usize, usize>::from_mut_words(&mut x, 3, 3 * WORD_BITS as u32);

    assert!(x.iter_rows(..).map(|x| &x.words).eq([
        [0, 0, 0].as_slice(),
        [0b1010, 0b1010, 0b1010].as_slice(),
        [0b0101, 0b0101, 0b0101].as_slice(),
    ]));
    assert!(x.iter_rows(0).map(|x| &x.words).eq([
        [0, 0, 0].as_slice(),
    ]));
    assert!(x.iter_rows(1).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010].as_slice(),
    ]));
    assert!(x.iter_rows(..2).map(|x| &x.words).eq([
        [0, 0, 0].as_slice(),
        [0b1010, 0b1010, 0b1010].as_slice(),
    ]));
    assert!(x.iter_rows(1..).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010].as_slice(),
        [0b0101, 0b0101, 0b0101].as_slice(),
    ]));
    assert!(x.iter_rows(1..2).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010].as_slice(),
    ]));

    x.copy_rows(0..1, 2);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
        ],
    );
    x.copy_rows(1, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
        ],);
    x.copy_rows(1.., 0);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
        ],);

    x.move_rows(0, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
        ],);
    x.move_rows(1..3, 0);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
        ],);
    x.move_rows(..2, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
        ],
    );
}

#[test]
#[rustfmt::skip]
fn row_iter_copy_move_4x5() {
    let mut x = [
        0, 0, 0, 0,
        0b1010, 0b1010, 0b1010, 0b1010,
        0b0101, 0b0101, 0b0101, 0b0101,
        0b1111, 0b1111, 0b1111, 0b1111,
        0b1001, 0b1001, 0b1001, 0b1001,
    ];
    let mut x = BitSlice2d::<'_, usize, usize>::from_mut_words(&mut x, 5, 4 * WORD_BITS as u32 - 1);

    assert!(x.iter_rows(..).map(|x| &x.words).eq([
        [0, 0, 0, 0].as_slice(),
        [0b1010, 0b1010, 0b1010, 0b1010].as_slice(),
        [0b0101, 0b0101, 0b0101, 0b0101].as_slice(),
        [0b1111, 0b1111, 0b1111, 0b1111].as_slice(),
        [0b1001, 0b1001, 0b1001, 0b1001].as_slice(),
    ]));
    assert!(x.iter_rows(0).map(|x| &x.words).eq([
        [0, 0, 0, 0].as_slice(),
    ]));
    assert!(x.iter_rows(1).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010, 0b1010].as_slice(),
    ]));
    assert!(x.iter_rows(..2).map(|x| &x.words).eq([
        [0, 0, 0, 0].as_slice(),
        [0b1010, 0b1010, 0b1010, 0b1010].as_slice(),
    ]));
    assert!(x.iter_rows(1..).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010, 0b1010].as_slice(),
        [0b0101, 0b0101, 0b0101, 0b0101].as_slice(),
        [0b1111, 0b1111, 0b1111, 0b1111].as_slice(),
        [0b1001, 0b1001, 0b1001, 0b1001].as_slice(),
    ]));
    assert!(x.iter_rows(1..2).map(|x| &x.words).eq([
        [0b1010, 0b1010, 0b1010, 0b1010].as_slice(),
    ]));

    x.copy_rows(0..1, 2);
    assert_eq!(
        x.words(),
        [
            0, 0, 0, 0,
            0b1010, 0b1010, 0b1010, 0b1010,
            0, 0, 0, 0,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.copy_rows(1, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0, 0,
            0b1010, 0b1010, 0b1010, 0b1010,
            0, 0, 0, 0,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.copy_rows(1.., 0);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010, 0b1010,
            0, 0, 0, 0,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );

    x.move_rows(0, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0, 0,
            0b1010, 0b1010, 0b1010, 0b1010,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.move_rows(1..3, 0);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010, 0b1010,
            0b1111, 0b1111, 0b1111, 0b1111,
            0, 0, 0, 0,
            0b1001, 0b1001, 0b1001, 0b1001,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.move_rows(..2, 1);
    assert_eq!(
        x.words(),
        [
            0, 0, 0, 0,
            0b1010, 0b1010, 0b1010, 0b1010,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.move_rows(1..4, 0);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010, 0b1010,
            0b1111, 0b1111, 0b1111, 0b1111,
            0b1001, 0b1001, 0b1001, 0b1001,
            0, 0, 0, 0,
            0b1001, 0b1001, 0b1001, 0b1001,
        ]
    );
    x.move_rows(2..=4, 1);
    assert_eq!(
        x.words(),
        [
            0b1010, 0b1010, 0b1010, 0b1010,
            0b1001, 0b1001, 0b1001, 0b1001,
            0, 0, 0, 0,
            0b1001, 0b1001, 0b1001, 0b1001,
            0, 0, 0, 0,
        ]
    );
}
