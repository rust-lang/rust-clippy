#![warn(clippy::missing_docs_in_private_items)]
#![allow(dead_code)]

pub fn public() {
    private();
}

#[expect(clippy::missing_docs_in_private_items)]
fn private() {}

// Don't lint for items in test modules
#[cfg(test)]
mod tests {
    #[test]
    fn test_in_mod() {
        fn inner_private() {}
    }

    fn private_in_mod() {}

    struct PrivateStructInMod;
}

// Don't lint for items in test functions
#[test]
fn test_function() {
    fn inner_private() {}
}

fn not_in_test_mod() {}
//~^ missing_docs_in_private_items

struct NotInTestModStruct;
//~^ missing_docs_in_private_items
