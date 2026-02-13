#[cfg_attr(all(), must_use = "note", deprecated)]
fn issue_12320() {}
//~^ must_use_unit

#[cfg_attr(all(), deprecated, doc = "foo", must_use = "note")]
fn issue_12320_2() {}
//~^ must_use_unit

fn main() {}
