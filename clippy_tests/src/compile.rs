use std::path::Path;
use failure::{Error, err_msg};
use std::process::Output;

fn compile(file: &Path) -> Result<Output, Error> {
    let example_name = file.file_stem()
        .ok_or_else(|| err_msg(format!("Couldn't get file name from {:?}", file)))?;

    let better_call_clippy = cmd!(
        "cargo", "check",
        "--example", example_name,
        "--message-format=json", "--quiet"
    );
    let res = better_call_clippy
        .env("RUSTC_WRAPPER", "clippy-driver")
        .env("CLIPPY_DISABLE_DOCS_LINKS", "true")
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()?;

    Ok(res)
}

pub fn get_json_errors(file: &Path) -> Result<String, Error> {
    let res = compile(file)?;
    let stderr = String::from_utf8(res.stderr)?;

    match res.status.code() {
        Some(0) | Some(1) | Some(101) => Ok(stderr),
        _ => Err(err_msg(
            format!("failed with status {:?}: {}", res.status.code(), stderr),
        ))
    }
}

pub fn without_errors(file: &Path) -> Result<(), Error> {
    let res = compile(file)?;

    match res.status.code() {
        Some(0) => Ok(()),
        _ => {
            debug!("file {:?} failed to compile:\n{}\n{}",
                file, String::from_utf8(res.stdout)?, String::from_utf8(res.stderr)?
            );
            Err(err_msg(format!(
                "failed with status {:?} (`env RUST_LOG=clippy_test=info` for more info)",
                res.status.code(),
            )))
        }
    }
}
