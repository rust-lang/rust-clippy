#![deny(clippy::useless_conversion)]
#![allow(dead_code, clippy::redundant_closure)]

#[derive(Debug)]
enum MyErr {
    IoErr,
}

impl From<std::io::Error> for MyErr {
    fn from(_: std::io::Error) -> Self {
        Self::IoErr
    }
}

fn map_err_from_fn(err: Result<(), std::io::Error>) -> Result<(), MyErr> {
    err.map_err(MyErr::from)?;
    //~^ useless_conversion
    Ok(())
}

fn map_err_from_closure(err: Result<(), std::io::Error>) -> Result<(), MyErr> {
    err.map_err(|e| MyErr::from(e))?;
    //~^ useless_conversion
    Ok(())
}

fn main() {}
