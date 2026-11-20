#![feature(rustc_private)]
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)]

use clippy_data_structures::GrowableBitSet2d;
use clippy_data_structures::bit_slice::WORD_BITS;

#[test]
#[rustfmt::skip]
fn row_iter_copy_move_3x3() {
    let mut x = GrowableBitSet2d::<usize, usize>::new(3 * WORD_BITS as u32);
    x.ensure_row(0).words.copy_from_slice(&[0, 0, 0]);
    x.ensure_row(1).words.copy_from_slice(&[0b1010, 0b1010, 0b1010]);
    x.ensure_row(2).words.copy_from_slice(&[0b0101, 0b0101, 0b0101]);

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
    x.copy_rows(1..3, 0);
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
    let mut x = GrowableBitSet2d::<usize, usize>::new(4 * WORD_BITS as u32 - 1);
    x.ensure_row(0).words.copy_from_slice(&[0, 0, 0, 0]);
    x.ensure_row(1).words.copy_from_slice(&[0b1010, 0b1010, 0b1010, 0b1010]);
    x.ensure_row(2).words.copy_from_slice(&[0b0101, 0b0101, 0b0101, 0b0101]);
    x.ensure_row(3).words.copy_from_slice(&[0b1111, 0b1111, 0b1111, 0b1111]);
    x.ensure_row(4).words.copy_from_slice(&[0b1001, 0b1001, 0b1001, 0b1001]);

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
    x.copy_rows(1..5, 0);
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

#[test]
#[rustfmt::skip]
fn row_copy_oob() {
    let mut x = GrowableBitSet2d::<usize, usize>::new(3 * WORD_BITS as u32 - 2);
    x.ensure_row(0).words.copy_from_slice(&[0, 0, 0]);
    x.ensure_row(1).words.copy_from_slice(&[0b1010, 0b1010, 0b1010]);
    x.ensure_row(2).words.copy_from_slice(&[0b0101, 0b0101, 0b0101]);
    x.ensure_row(3).words.copy_from_slice(&[0b1111, 0b1111, 0b1111]);

    x.copy_rows(0, 4);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
        ]
    );
    x.copy_rows(1, 5);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
        ]
    );
    x.copy_rows(..3, 7);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
        ]
    );
    x.copy_rows(1..3, 9);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
        ]
    );
    x.copy_rows(11..15, 0);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
        ]
    );
    x.copy_rows(9..12, 3);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
        ]
    );
    x.copy_rows(20..30, 9);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
        ]
    );
    x.copy_rows(20, 30);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
        ]
    );
}

#[test]
#[rustfmt::skip]
fn row_move_oob() {
    let mut x = GrowableBitSet2d::<usize, usize>::new(3 * WORD_BITS as u32 - 2);
    x.ensure_row(0).words.copy_from_slice(&[0, 0, 0]);
    x.ensure_row(1).words.copy_from_slice(&[0b1010, 0b1010, 0b1010]);
    x.ensure_row(2).words.copy_from_slice(&[0b0101, 0b0101, 0b0101]);
    x.ensure_row(3).words.copy_from_slice(&[0b1111, 0b1111, 0b1111]);

    x.move_rows(0, 4);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
        ]
    );
    x.move_rows(1, 5);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0b0101, 0b0101, 0b0101,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
        ]
    );
    x.move_rows(..3, 7);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b0101, 0b0101, 0b0101,
        ]
    );
    x.ensure_row(1).words.fill(1);
    x.move_rows(1..3, 9);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            1, 1, 1,
            0, 0, 0,
        ]
    );
    x.ensure_row(0).words.fill(0b10);
    x.move_rows(11..15, 0);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0b1010, 0b1010, 0b1010,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            1, 1, 1,
            0, 0, 0,
        ]
    );
    x.ensure_row(10).words.fill(0b1111);
    x.move_rows(9..12, 3);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            1, 1, 1,
            0b1111, 0b1111, 0b1111,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
        ]
    );
    x.move_rows(20..40, 4);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            1, 1, 1,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
        ]
    );
    x.move_rows(20, 30);
    assert_eq!(
        x.words(),
        [
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            1, 1, 1,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
        ]
    );
}
