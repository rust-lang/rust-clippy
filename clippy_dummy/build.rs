use term::color::{GREEN, RED, WHITE};
use term::{Attr, Error, Result};

fn main() {
    if foo().is_err() {
        eprintln!(
            "error: `clippy_config` is not available through crates.io\n\n\
             help: please use it as a git dependency"
        );
    }
    std::process::exit(1);
}

fn foo() -> Result<()> {
    let mut t = term::stderr().ok_or(Error::NotSupported)?;

    t.attr(Attr::Bold)?;
    t.fg(RED)?;
    write!(t, "\nerror: ")?;

    t.reset()?;
    t.fg(WHITE)?;
    writeln!(t, "`clippy_config` is not available through crates.io\n")?;

    t.attr(Attr::Bold)?;
    t.fg(GREEN)?;
    write!(t, "help: ")?;

    t.reset()?;
    t.fg(WHITE)?;
    write!(t, "please use it as a git dependency")?;

    t.reset()?;
    Ok(())
}
