use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::process;

use crate::new_lint::get_stabilization_version;

#[derive(serde::Deserialize)]
struct Lint {
    id: String,
    version: String,
}

fn load_metadata(metadata: &Path) -> HashMap<String, String> {
    let lints: Vec<Lint> = serde_json::from_reader(File::open(metadata).unwrap()).unwrap();
    lints.into_iter().map(|lint| (lint.id, lint.version)).collect()
}

pub fn check_lint_version(old_metadata: &Path, new_metadata: &Path) {
    let stabilization_version = get_stabilization_version();
    let old_lints = load_metadata(old_metadata);
    let mut new_lints = load_metadata(new_metadata)
        .into_iter()
        .filter(|(name, _)| !old_lints.contains_key(name))
        .collect::<Vec<_>>();
    if new_lints.is_empty() {
        println!("No new lints");
        return;
    }
    new_lints.sort_unstable();
    let mut error = false;
    println!("New lints:");
    for (name, version) in new_lints {
        if version == stabilization_version {
            println!("  - {name}");
        } else {
            println!("  - {name}: lint declares version {version}, stabilization version is {stabilization_version}");
            error = true;
        }
    }
    if error {
        process::exit(1);
    }
}
