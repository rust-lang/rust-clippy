#![warn(clippy::missing_examples_doc)]

pub fn pub_fn_missing_docs() {
    //~^ missing_examples_doc
    unimplemented!();
}

/// Docs without examples
pub fn pub_fn_missing_examples() {
    //~^ missing_examples_doc
    unimplemented!();
}

/// Docs with examples
///
/// # Examples
///
/// ```
/// pub_fn_missing_examples()
/// ```
pub fn pub_fn_with_examples() {
    unimplemented!();
}

fn priv_fn_missing_docs() {
    unimplemented!();
}

#[doc(hidden)]
pub fn hidden_fn_missing_docs() {
    unimplemented!();
}
