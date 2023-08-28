//// borked doc comment on the first line. doesn't combust!
//~^ ERROR: this item has comments with 4 forward slashes (`////`). These look like doc co
//~| NOTE: `-D clippy::four-forward-slashes` implied by `-D warnings`
fn a() {}

// This test's entire purpose is to make sure we don't panic if the comment with four slashes
// extends to the first line of the file. This is likely pretty rare in production, but an ICE is an
// ICE.
