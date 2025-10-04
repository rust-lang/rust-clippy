// https://github.com/rust-lang/rust-clippy/issues/15353
#![warn(clippy::missing_panics_doc, clippy::missing_safety_doc, clippy::missing_errors_doc)]

pub struct Error;

/// <div>
/// # Panics
///
/// Here's where some panic docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub fn panicking1() {
    //~^ missing_panics_doc
    panic!();
}

/// > <div>
/// > # Panics
/// >
/// > Here's where some panic docs are supposed to appear,
/// > but don't, because of the div.
/// >
/// > Make sure that the generated suggestion puts a blank
/// > line between the header and the HTML.
/// >
/// > </div>
pub fn panicking1blockquote() {
    //~^ missing_panics_doc
    panic!();
}

#[rustfmt::skip]
/// > - <div>
/// >   # Panics
/// >
/// >   Here's where some panic docs are supposed to appear,
/// >   but don't, because of the div.
//  >
/// >   </div>
/// >
/// > - Make sure that the generated suggestion puts a blank
/// >   line between the header and the HTML.
pub fn panicking1blockquotelist() {
    //~^ missing_panics_doc
    panic!();
}

/// <div>
/// # Safety #
///
/// Here's where some safety docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub unsafe fn safety1() {
    //~^ missing_safety_doc
}

/// <div>
/// # Errors ##
///
/// Here's where some error docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub fn errors1() -> Result<(), Error> {
    //~^ missing_errors_doc
    Ok(())
}

/// <div>
/// # Panics
///
/// # Panics
/// Here's one where the panic docs actually exist.
/// Make sure there's no warning, and no suggestion.
///
/// </div>
pub fn panicking2() {
    panic!();
}

/// <div>
/// # Safety
///
/// # Safety
/// Here's one where the safety docs actually exist.
/// Make sure there's no warning, and no suggestion.
///
/// </div>
pub unsafe fn safety2() {}

/// <div>
/// # Errors
///
/// # Errors
/// Here's one where the error docs actually exist.
/// Make sure there's no warning, and no suggestion.
///
/// </div>
pub fn errors2() -> Result<(), Error> {
    Ok(())
}

/// <div>
/// # Panics
///
/// Here's one where no panic docs should appear at all.
/// Make sure there's no warning, and no suggestion.
///
/// </div>
pub fn no_panicking1() {}

/// <div>
/// # Safety #
///
/// Here's one where no safety docs should appear at all.
/// Make sure there's no warning, and no suggestion.
///
/// </div>
pub fn no_safety1() {}

/// <div>
/// # Errors ##
///
/// Here's one where no error docs should appear at all.
/// Make sure there's no warning, and no suggestion.
//
/// </div>
pub fn no_errors1() {}

/// <div>
/// Panics
/// ==
///
/// Here's where some panic docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub fn panicking1b() {
    //~^ missing_panics_doc
    panic!();
}

/// <div>
/// Safety
/// ===
///
/// Here's where some safety docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub unsafe fn safety1b() {
    //~^ missing_safety_doc
}

/// <div>
/// Errors
/// ====
///
/// Here's where some error docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure that the generated suggestion puts a blank
/// line between the header and the HTML.
///
/// </div>
pub fn errors1b() -> Result<(), Error> {
    //~^ missing_errors_doc
    Ok(())
}

/// <div>
/// Panics
/// ==
///
/// Panics
/// ==
/// Here's one where the panic docs actually exist.
/// Make sure there's no warnings or anything.
///
/// </div>
pub fn panicking2b() {
    panic!();
}

/// <div>
/// Safety
/// ==
///
/// Safety
/// ==
/// Here's one where the safety docs actually exist.
///
/// </div>
pub unsafe fn safety2b() {}

/// <div>
/// Panics
/// ==
///
/// Here's one where no panic docs should appear at all.
/// Make sure there's no warnings or suggestions.
///
/// </div>
pub fn no_panicking1b() {}

/// <div>
/// Safety
/// ==
///
/// Here's one where no safety docs should appear at all.
/// Make sure there's no warnings or suggestions.
///
/// </div>
pub fn no_safety1b() {}

/// <div>
/// # Safety
///
/// # Safety
///
/// Here's one where no safety docs should not appear,
/// but do. Make sure there's no spurrious suggestions
/// to add a blank line between the div and the
/// suspicious header.
///
/// </div>
pub fn spurrious_safety1() {}

/// <div>
/// Safety
/// ==
///
/// Safety
/// ==
///
/// Here's one where no safety docs should not appear,
/// but do. Make sure there's no spurrious suggestions
/// to add a blank line between the div and the
/// suspicious header.
///
/// </div>
pub fn spurrious_safety1b() {}

// == block ==

/**
  <div>
  ## Panics

  Here's where some panic docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub fn panicking3() {
    //~^ missing_panics_doc
    panic!();
}

#[rustfmt::skip]
/**
  - <div>
    ## Panics

    Here's where some panic docs are supposed to appear,
    but don't, because of the div.

    Make sure that the generated suggestion puts a blank
    line between the header and the HTML.

    </div>
*/
pub fn panicking3list() {
    //~^ missing_panics_doc
    panic!();
}

#[rustfmt::skip]
/**
  - > <div>
    > ## Panics
    >
    > Here's where some panic docs are supposed to appear,
    > but don't, because of the div.
    >
    > Make sure that the generated suggestion puts a blank
    > line between the header and the HTML.
    >
    > </div>
*/
pub fn panicking3listblockquote() {
    //~^ missing_panics_doc
    panic!();
}

/**
  <div>
  ## Safety #

  Here's where some safety docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub unsafe fn safety3() {
    //~^ missing_safety_doc
}

/**
  <div>
  ## Errors ##

  Here's where some error docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub fn errors3() -> Result<(), Error> {
    //~^ missing_errors_doc
    Ok(())
}

/**
  <div>
  ## Panics

  ## Panics
  Here's one where the panic docs actually exist.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub fn panicking4() {
    panic!();
}

/**
  <div>
  ## Safety

  ## Safety
  Here's one where the safety docs actually exist.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub unsafe fn safety4() {}

/**
  <div>
  ## Errors

  ## Errors
  Here's one where the error docs actually exist.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub fn errors4() -> Result<(), Error> {
    Ok(())
}

/**
  <div>
  ## Panics

  Here's one where no panic docs should appear at all.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub fn no_panicking2() {}

/**
  <div>
  ## Safety

  Here's one where no safety docs should appear at all.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub fn no_safety2() {}

/**
  <div>
  ## Errors

  Here's one where no error docs should appear at all.
  Make sure there's no warning, and no suggestion.

  </div>
*/
pub fn no_errors2() {}

/**
  <div>
  Panics
  --

  Here's where some panic docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub fn panicking3b() {
    //~^ missing_panics_doc
    panic!();
}

/**
  <div>
  Safety
  ---

  Here's where some safety docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub unsafe fn safety3b() {
    //~^ missing_safety_doc
}

/**
  <div>
  Errors
  ----

  Here's where some error docs are supposed to appear,
  but don't, because of the div.

  Make sure that the generated suggestion puts a blank
  line between the header and the HTML.

  </div>
*/
pub fn errors3b() -> Result<(), Error> {
    //~^ missing_errors_doc
    Ok(())
}

/**
  <div>
  Panics
  --

  Panics
  --
  Here's one where the panic docs actually exist.
  Make sure there's no warnings or anything.

  </div>
*/
pub fn panicking4b() {
    panic!();
}

/**
  <div>
  Safety
  --

  Safety
  --
  Here's one where the safety docs actually exist.

  </div>
*/
pub unsafe fn safety4b() {}

/**
  <div>
  Panics
  --

  Here's one where no panic docs should appear at all.
  Make sure there's no warnings or suggestions.

  </div>
*/
pub fn no_panicking3b() {}

/**
  <div>
  Safety
  --

  Here's one where no safety docs should appear at all.
  Make sure there's no warnings or suggestions.

  </div>
*/
pub fn no_safety3b() {}

/**
  <div>
  ## Safety

  ## Safety

  Here's one where no safety docs should not appear,
  but do. Make sure there's no spurrious suggestions
  to add a blank line between the div and the
  suspicious header.

  </div>
*/
pub fn spurrious_safety3() {}

/**
  <div>
  Safety
  --

  Safety
  --

  Here's one where no safety docs should not appear,
  but do. Make sure there's no spurrious suggestions
  to add a blank line between the div and the
  suspicious header.

  </div>
*/
pub fn spurrious_safety3b() {}
