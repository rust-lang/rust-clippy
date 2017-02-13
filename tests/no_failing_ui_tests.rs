use std::io::BufRead;

#[test]
fn check_whether_ui_tests_error() {
    let mut success = true;
    for file in std::fs::read_dir("tests/ui").unwrap() {
        let path = file.unwrap().path();
        if let Some(ext) = path.extension() {
            if ext == "stderr" {
                for line in std::io::BufReader::new(std::fs::File::open(&path).unwrap()).lines() {
                    let line = line.unwrap();
                    if line.starts_with("error:") {
                        success = false;
                        println!("ui test `{}` has errors, please change all lints to `warn`",
                                 path.to_str().unwrap());
                        println!("{}", line);
                    }
                }
            }
        }
    }
    assert!(success, "some ui tests are erroring");
}

#[test]
fn check_whether_ui_error_tests_dont_error() {
    let mut success = true;
    for file in std::fs::read_dir("tests/ui-error").unwrap() {
        let path = file.unwrap().path();
        if let Some(ext) = path.extension() {
            if ext == "stderr" {
                if !std::io::BufReader::new(std::fs::File::open(&path).unwrap())
                    .lines()
                    .any(|line| line.unwrap().starts_with("error:")) {
                    success = false;
                    println!("ui-error test `{}` has no errors", path.to_str().unwrap());
                }
            }
        }
    }
    assert!(success, "some ui-error tests are not erroring");
}
