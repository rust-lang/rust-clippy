#![warn(clippy::unused_enumerate_value)]

fn main() {
    let mut array = [1, 2, 3];
    for (index, _) in array.iter_mut().enumerate() {
        //~^ unused_enumerate_value
        todo!();
    }

    let my_iter = vec![1, 2, 3].into_iter();
    for (index, _) in my_iter.enumerate() {
        //~^ unused_enumerate_value
        todo!();
    }

    let another_iter = vec![1, 2, 3].into_iter();
    for (index, _) in another_iter.enumerate().map(|(index, x)| (index, x + 1)) {
        todo!();
    }
}
