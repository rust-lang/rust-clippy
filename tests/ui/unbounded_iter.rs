#![allow(clippy::all, clippy::pedantic)]
#![warn(clippy::unbounded_iter)]

fn ranges_and_limited_use() {
    let _ = (0usize..).all(|x| x < 42);
    //~^ unbounded_iter
    let _ = (0usize..).count();
    //~^ unbounded_iter

    let _ = (0usize..42).all(|x| x < 42);
    let _ = (0usize..=42).count();
    let _iter = 0usize..;
    let _iter = (0usize..).map(|x| x + 1);
    let _ = (0usize..).next();
    let _ = (0usize..).map(|x| x + 1).next();

    let mut iter = 0usize..;
    let _ = iter.next();
    let _ = iter.next();
}

fn zip() {
    let _ = (0usize..).zip(0usize..).count();
    //~^ unbounded_iter

    // Either bounded side bounds the zip
    let _ = (0usize..).zip(0usize..42).count();
    let _ = (0usize..42).zip(0usize..).count();
    let _ = (0usize..42).zip(0usize..42).count();
    let _ = (0usize..).zip((0usize..).take(42)).count();
}

fn chain() {
    let _ = (0usize..).chain(0usize..).count();
    //~^ unbounded_iter
    let _ = (0usize..).chain(0usize..42).count();
    //~^ unbounded_iter
    let _ = (0usize..42).chain(0usize..).count();
    //~^ unbounded_iter

    let _ = (0usize..42).chain(0usize..42).count();
}

fn flat_map_and_flatten() {
    let _ = (0usize..3).flat_map(|_| 0usize..).count();
    //~^ unbounded_iter
    let _ = [0usize..].into_iter().flatten().count();
    //~^ unbounded_iter

    // Bounding the outer iterator doesn't bound the expanded output
    let _ = (0usize..).take(3).flat_map(|_| 0usize..).count();
    //~^ unbounded_iter
    let _ = (0usize..).map(|_| 0usize..).take(3).flatten().count();
    //~^ unbounded_iter

    let _ = (0usize..3).flat_map(|_| 0usize..).take(42).count();
    let _ = [0usize..].into_iter().flatten().take(42).count();
    let _iter = (0usize..3).flat_map(|_| 0usize..);
    let _iter = [0usize..].into_iter().flatten();

    // The current implementation treats even finite expansions as unbounded
    let _ = (0usize..3).flat_map(|_| 0usize..2).count();
    //~^ unbounded_iter
    let _ = [0usize..2].into_iter().flatten().count();
    //~^ unbounded_iter
}

fn take() {
    // Do not lint the inner open-ended range
    let _ = (0usize..).take(42).all(|x| x < 42);
    let _ = (0usize..).take(0).count();
    let _ = (0usize..).take(42).count();
    let _ = (0usize..).chain(0usize..).take(42).count();

    // Take only bounds the first part of the chain
    let _ = (0usize..).take(42).chain(0usize..).count();
    //~^ unbounded_iter
}

fn other_adapters() {
    let _ = (0usize..).map(|x| x + 1).count();
    //~^ unbounded_iter
    let _ = (0usize..).skip(10).count();
    //~^ unbounded_iter
    let _ = (0usize..).into_iter().count();
    //~^ unbounded_iter

    let _ = (0usize..42).map(|x| x + 1).count();
    let _ = (0usize..42).skip(10).count();
    let _ = (0usize..).take(42).map(|x| x + 1).count();
}

fn blocks() {
    let _ = ({
        //~^ unbounded_iter
        std::hint::black_box(());
        0usize..
    })
    .count();

    let _ = ({
        std::hint::black_box(());
        0usize..42
    })
    .count();

    let _ = ({
        std::hint::black_box(());
        (0usize..).take(42)
    })
    .count();
}

fn references() {
    let _ = (&mut (0usize..)).count();
    //~^ unbounded_iter

    let _ = (&mut (0usize..42)).count();
    let _ = (&mut (0usize..).take(42)).count();
}

fn other_expression_kinds() {
    let _ = std::iter::empty::<usize>().count();
    let _ = [1usize, 2, 3].into_iter().count();

    let iter = 0usize..42;
    let _ = iter.count();
}

fn consuming_methods() {
    let _ = (0usize..).max();
    //~^ unbounded_iter
    let _ = (0usize..).sum::<usize>();
    //~^ unbounded_iter
    let _ = (0usize..).collect::<Vec<_>>();
    //~^ unbounded_iter
    let _ = (0usize..).fold(0usize, |acc, x| acc ^ x);
    //~^ unbounded_iter
    let _ = (0usize..).last();
    //~^ unbounded_iter
    let _: (Vec<_>, Vec<_>) = (0usize..).map(|x| (x, x)).unzip();
    //~^ unbounded_iter

    let _ = (0usize..).take(42).max();
    let _ = (0usize..).take(42).sum::<usize>();
    let _ = (0usize..).take(42).collect::<Vec<_>>();
    let _ = (0usize..).take(42).fold(0usize, |acc, x| acc ^ x);
    let _ = (0usize..).take(42).last();
    let _: (Vec<_>, Vec<_>) = (0usize..).take(42).map(|x| (x, x)).unzip();

    let _ = (0usize..42).max();
    let _ = (0usize..42).sum::<usize>();
    let _ = (0usize..42).collect::<Vec<_>>();
}

fn diagnostic_endpoints() {
    // Only underline through max(), sum(), or collect(), excluding the trailing call
    let _ = (0usize..)
        //~^ unbounded_iter
        .map(|x| x + 1)
        .max()
        .unwrap_or(0);

    let _ = (0usize..)
        //~^ unbounded_iter
        .sum::<usize>()
        .max(42);

    let _ = (0usize..)
        //~^ unbounded_iter
        .collect::<Vec<_>>()
        .len();
}

fn consumption_boundaries() {
    // Only the inner consumer should lint, not the final count()
    let _ = (0usize..)
        //~^ unbounded_iter
        .max()
        .into_iter()
        .count();

    let _ = (0usize..)
        //~^ unbounded_iter
        .collect::<Vec<_>>()
        .into_iter()
        .count();

    let _ = (0usize..).take(42).collect::<Vec<_>>().into_iter().count();
    let _ = (0usize..).next().into_iter().count();
}

struct Empty {
    _marker: (),
}

impl Iterator for Empty {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

fn non_range_struct() {
    let _ = Empty { _marker: () }.count();
}

fn main() {}
