#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate failure_derive;
extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate duct;
extern crate tempdir;
extern crate env_logger;
extern crate rayon;
extern crate rustfix;

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use failure::{Error, ResultExt, err_msg};
use rayon::prelude::*;

mod compile;
mod fix;
mod diff;
mod human;
mod test_results;
use test_results::TestResults;

fn read_file(path: &Path) -> Result<String, Error> {
    use std::io::Read;

    let mut buffer = String::new();
    let mut file = fs::File::open(path)?;
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn rename(file: &Path, suffix: &str) -> Result<PathBuf, Error> {
    let name = file.file_stem()
        .ok_or_else(|| err_msg(format!("Couldn't get file stem from {:?}", file)))?
        .to_str()
        .ok_or_else(|| err_msg(format!("Couldn't parse file stem from {:?}", file)))?;

    Ok(file.with_file_name(&format!("{}--{}.rs", name, suffix)))
}

#[derive(Fail, Debug)]
#[fail(display = "Could not test suggestions, there was no assertion file")]
struct NoSuggestionsTestFile;

#[derive(Fail, Debug)]
#[fail(display = "Could not test human readable error messages, there was no assertion file")]
struct NoUiTestFile;

#[derive(Fail, Debug)]
#[fail(display = "Mismatch between expected and generated fixed file")]
struct FixedFileMismatch;

#[derive(Fail, Debug)]
#[fail(display = "Mismatch between expected and generated diagnostics output")]
struct StderrFileMismatch;

fn test_rustfix_with_file<P: AsRef<Path>>(file: P) -> Result<(), Error> {
    let file: &Path = file.as_ref();
    let fixed_file = rename(file, "fixed")?;

    debug!("testing: {:?}", file);

    let code = read_file(file)?;
    debug!("compiling... {:?}", file);
    let errors = compile::get_json_errors(file)?;

    let diagnostics = human::ui_from_json(&errors);

    if std::env::var("RECORD_NEW_DIAGNOSTICS").is_ok() {
        use std::io::Write;
        let diagnostics_recording = rename(file, "recorded")?.with_extension("stderr");
        let mut recorded_diagnostics = fs::File::create(&diagnostics_recording)?;
        recorded_diagnostics.write_all(diagnostics.as_bytes())?;
        debug!("wrote recorded diagnostics for {:?} to {:?}", file, diagnostics_recording);
    }

    let ui_file = file.with_extension("stderr");
    let expected_diagnostics = read_file(&ui_file).map_err(|_| NoUiTestFile)?;

    debug!("comparing diagnostics for {:?}", file);
    if diagnostics.trim() != expected_diagnostics.trim() {
        use log::Level::Debug;
        if log_enabled!(Debug) {
            debug!(
                "Difference between generated and expected diagnostics:\n{}",
                diff::render(&diagnostics, &expected_diagnostics)?,
            );
        }
        Err(StderrFileMismatch)?;
    };

    debug!("collecting suggestions for {:?}", file);
    let suggestions = rustfix::get_suggestions_from_json(&errors, &HashSet::new());

    debug!("applying suggestions for {:?}", file);
    let fixed = fix::apply_suggestions(&code, &suggestions)?;

    if std::env::var("RUSTFIX_PLS_FIX_AND_RECORD_MY_CODE").is_ok() {
        use std::io::Write;
        let fixes_recording = rename(file, "recorded")?;
        let mut recorded_rust = fs::File::create(&fixes_recording)?;
        recorded_rust.write_all(fixed.as_bytes())?;
        debug!("wrote recorded fixes for {:?} to {:?}", file, fixes_recording);
    }

    let expected_fixed = read_file(&fixed_file).map_err(|_| NoSuggestionsTestFile)?;
    if fixed.trim() != expected_fixed.trim() {
        use log::Level::Debug;
        if log_enabled!(Debug) {
            debug!(
                "Difference between file produced by rustfix \
                and expected fixed file:\n{}",
                diff::render(&fixed, &expected_fixed)?,
            );
        }
        Err(FixedFileMismatch)?;
    };

    debug!("compiling fixed file {:?}", fixed_file);
    compile::without_errors(&fixed_file)?;

    Ok(())
}

fn get_fixture_files() -> Result<Vec<PathBuf>, Error> {
    Ok(fs::read_dir("./examples").context("couldn't read examples dir")?
        .into_iter()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let x = p.to_string_lossy();
            x.ends_with(".rs") && !x.ends_with("--fixed.rs") && !x.ends_with("--recorded.rs")
        })
        .collect())
}

fn run(file: &Path) -> TestResults {
    match test_rustfix_with_file(&file) {
        Ok(_) => {
            info!("passed: {:?}", file);
            TestResults::passed()
        },
        Err(e) => {
            match e.downcast::<NoSuggestionsTestFile>() {
                Ok(e) => {
                    info!("ignored: {:?} (no fixed file)", file);
                    debug!("{}", e);
                    TestResults::ignored()
                },
                Err(e) => {
                    warn!("failed: {:?}", file);
                    debug!("{}", e);
                    TestResults::failed()
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    let files = get_fixture_files().unwrap();

    let res = files.par_iter()
        // .take(10)
        .map(|path| run(path))
        .reduce(TestResults::default, |a, b| a + b);

    println!("{}", res);
}
