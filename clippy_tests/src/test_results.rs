#[derive(Debug, Default, Clone, Copy)]
pub struct TestResults {
    passed: i32,
    failed: i32,
    ignored: i32,
}

impl TestResults {
    pub fn passed() -> TestResults {
        TestResults { passed: 1, ..TestResults::default() }
    }

    pub fn failed() -> TestResults {
        TestResults { failed: 1, ..TestResults::default() }
    }

    pub fn ignored() -> TestResults {
        TestResults { ignored: 1, ..TestResults::default() }
    }
}

impl ::std::ops::Add for TestResults {
    type Output = TestResults;

    fn add(self, other: Self) -> Self {
        TestResults {
            passed: self.passed + other.passed,
            failed: self.failed + other.failed,
            ignored: self.ignored + other.ignored,
        }
    }
}

impl ::std::fmt::Display for TestResults {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let res = if self.failed > 0 { "failed" } else { "ok" };

        write!(f,
            "test result: {}. {} passed; {} failed; {} ignored;",
            res, self.passed, self.failed, self.ignored,
        )
    }
}
