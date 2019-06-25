#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::unnecessary_filter_map)]

fn main() {
    let options: Vec<Option<i8>> = vec![Some(5), None, Some(6)];
    let results = ["1", "lol", "3", "NaN", "5"];

    // validate iterator of values triggers filter_map + map lint
    let _: Vec<_> = vec![5_i8; 6].into_iter()
        .filter_map(|x| x.checked_mul(2))
        .map(|x| x.checked_mul(2))
        .collect();

    // validate iterator of options triggers filter + map lint
    let _: Vec<i8> = options.clone().into_iter()
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();

    // validate iterator of options triggers filter + flat_map lint
    let _: Vec<Option<i8>> = std::iter::repeat(options.clone())
        .take(5)
        .map(|v| Some(v.into_iter()))
        .filter(|x| x.is_some())
        .flat_map(|x| x.unwrap())
        .collect();

    // validate iterator of results triggers filter + map lint
    let _: Vec<i8> =  results.iter()
        .map(|s| s.parse())
        .filter(|s| s.is_ok())
        .map(|s| s.unwrap())
        .collect();

    // validate iterator of values **does not** trigger filter + map lint
    let _: Vec<String> =  results.iter()
        .filter(|s| s.len() > 3)
        .map(|s| format!("{}{}", s, s))
        .collect();

    // validate iterator of values **does not** trigger filter + flat_map lint
    let _: String = results.iter()
        .filter(|i| i.len() > 1)
        .flat_map(|s| s.chars())
        .collect();

    // validate filter_map + flat_map **does not** trigger linter
    let _: Vec<_> = vec![5_i8; 6]
        .into_iter()
        .filter_map(|x| x.checked_mul(2))
        .flat_map(|x| x.checked_mul(2))
        .collect();
}
