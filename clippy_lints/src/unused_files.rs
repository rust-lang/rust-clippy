extern crate syntax;

use rustc::lint::{EarlyLintPass, EarlyContext, LintContext, LintPass, LintArray};
use syntax::ast;

use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;

declare_lint!(
    pub UNUSED_FILES,
    Allow,
    "warns about unused Rust files");

pub struct UnusedFilesPass;


impl LintPass for UnusedFilesPass {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_FILES)
    }
}

impl EarlyLintPass for UnusedFilesPass {
    fn check_crate(&mut self, ctx: &EarlyContext, _: &ast::Crate) {
        let cm = ctx.sess.codemap();

        let mut visited: HashSet<String>  = HashSet::new();
        for file in cm.files.borrow().iter() {
            let path = ctx.sess.working_dir.join(Path::new(&file.name));
            visited.insert(path.to_str().unwrap().to_string());
        }
        if let Some(ref path) = ctx.sess.local_crate_source_file {
            let mut dir = path.clone();
            dir.pop();

            let mut rs_files = HashSet::new();
            for entry in WalkDir::new(dir) {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext == "rs" {
                                rs_files.insert(path.to_str().unwrap().to_string());
                            }
                        }
                    },
                    Err(e) => {
                        ctx.lint(UNUSED_FILES,
                             &format!("Error walking crate directory: {:?}", e));
                    }
                }
            }

            let diff: HashSet<String> = rs_files.difference(&visited).cloned().collect();

            if diff.len() > 0 {
                let files: Vec<String> = diff.iter().map(|s| s.clone()).collect::<Vec<String>>();
                ctx.lint(UNUSED_FILES,
                         &format!("Found {} unused files:\n{}\n", diff.len(), files.join("\n")));
            }
        }
    }
}
