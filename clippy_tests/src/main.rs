extern crate serde_json;
#[macro_use] extern crate failure_derive;
extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate duct;
#[macro_use] extern crate pretty_assertions;
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
use compile::*;
mod fix;

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
pub struct NoSuggestionsTestFile;

#[derive(Fail, Debug)]
#[fail(display = "Could not test human readable error messages, there was no assertion file")]
pub struct NoUiTestFile;

fn test_rustfix_with_file<P: AsRef<Path>>(file: P) -> Result<(), Error> {
    let file: &Path = file.as_ref();
    let fixed_file = rename(file, "fixed")?;

    debug!("testing: {:?}", file);

    let code = read_file(file)?;
    debug!("compiling... {:?}", file);
    let errors = compile_and_get_json_errors(file)?;
    debug!("collecting suggestions for {:?}", file);
    let suggestions = rustfix::get_suggestions_from_json(&errors, &HashSet::new());

    let mut fixed = code.clone();

    debug!("applying suggestions for {:?}", file);
    for sug in suggestions.into_iter().rev() {
        trace!("{:?}", sug);
        for sol in sug.solutions {
            trace!("{:?}", sol);
            for r in sol.replacements {
                debug!("replaced.");
                trace!("{:?}", r);
                fixed = fix::apply_suggestion(&mut fixed, &r)?;
            }
        }
    }

    if std::env::var("RUSTFIX_PLS_FIX_AND_RECORD_MY_CODE").is_ok() {
        use std::io::Write;
        let fixes_recording = rename(file, "recorded")?;
        let mut recorded_rust = fs::File::create(&fixes_recording)?;
        recorded_rust.write_all(fixed.as_bytes())?;
        debug!("wrote recorded fixes for {:?} to {:?}", file, fixes_recording);
    }

    let expected_fixed = read_file(&fixed_file).map_err(|_| NoSuggestionsTestFile)?;
    assert_eq!(fixed.trim(), expected_fixed.trim(), "file doesn't look fixed");

    debug!("compiling fixed file {:?}", fixed_file);
    compiles_without_errors(&fixed_file)?;

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

fn main() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    env_logger::init();
    let files = get_fixture_files().unwrap();

    let passed = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let ignored = AtomicUsize::new(0);

    files.par_iter().for_each(|file| {
        match test_rustfix_with_file(&file) {
            Ok(_) => {
                info!("passed: {:?}", file);
                passed.fetch_add(1, Ordering::SeqCst);
            },
            Err(e) => {
                match e.downcast::<NoSuggestionsTestFile>() {
                    Ok(e) => {
                        info!("ignored: {:?} (no fixed file)", file);
                        debug!("{:?}", e);
                        ignored.fetch_add(1, Ordering::SeqCst);
                    },
                    Err(e) => {
                        warn!("failed: {:?}", file);
                        debug!("{:?}", e);
                        failed.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        }
    });

    let passed = passed.into_inner();
    let failed = failed.into_inner();
    let ignored = ignored.into_inner();
    let res = if failed > 0 { "failed" } else { "ok" };
    println!(
        "test result: {}. {} passed; {} failed; {} ignored;",
        res, passed, failed, ignored,
    );
}
