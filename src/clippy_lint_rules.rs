use rustc_data_structures::fx::FxHashMap;

fn level_from_flag(flag: &str) -> Option<&'static str> {
    match flag {
        "-A" | "--allow" => Some("allow"),
        "-W" | "--warn" => Some("warn"),
        "-D" | "--deny" => Some("deny"),
        "-F" | "--forbid" => Some("forbid"),
        _ => None,
    }
}

fn flag_from_level(level: &str) -> Option<&'static str> {
    match level {
        "allow" => Some("-A"),
        "warn" => Some("-W"),
        "deny" => Some("-D"),
        "forbid" => Some("-F"),
        _ => None,
    }
}

fn read_clippy_toml_overrides() -> Vec<(String, String, i64)> {
    if let Ok(conf_path) = clippy_config::lookup_conf_file() {
        if let Some(lints) = clippy_config::read_lints_from_conf_path(&Ok(conf_path)) {
            return lints
                .clippy
                .into_iter()
                .map(|(n, c)| (format!("clippy::{}", n), c.level().to_string(), c.priority()))
                .collect();
        }
    }
    Vec::new()
}

fn find_conf_file_from(start_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    const CONFIG_FILE_NAMES: [&str; 2] = [".clippy.toml", "clippy.toml"];
    let mut current = start_dir.canonicalize().ok()?;
    loop {
        for name in &CONFIG_FILE_NAMES {
            let candidate = current.join(name);
            if let Ok(md) = std::fs::metadata(&candidate) {
                if md.is_file() {
                    return Some(candidate);
                }
            }
        }
        if !current.pop() {
            return None;
        }
    }
}

fn read_clippy_toml_overrides_from(start_dir: &std::path::Path) -> Vec<(String, String, i64)> {
    if let Some(conf) = find_conf_file_from(start_dir) {
        if let Some(lints) = clippy_config::read_lints_from_conf_path(&Ok((Some(conf), vec![]))) {
            return lints
                .clippy
                .into_iter()
                .map(|(n, c)| (format!("clippy::{}", n), c.level().to_string(), c.priority()))
                .collect();
        }
    }
    Vec::new()
}

fn parse_existing_lint_args(args: &[String]) -> FxHashMap<String, (String, i64)> {
    let mut m = FxHashMap::default();
    let mut i = 0;
    let mut push = |lvl: &str, name: String| {
        if name.starts_with("clippy::") {
            m.insert(name, (lvl.to_string(), 0));
        }
    };
    while i < args.len() {
        let a = &args[i];
        if let Some((f, v)) = a.split_once('=') {
            if let Some(l) = level_from_flag(f) {
                push(l, v.to_string());
                i += 1;
                continue;
            }
        }
        if a.len() > 2 {
            let (h, t) = a.split_at(2);
            if let Some(l) = level_from_flag(h) {
                if t.starts_with("clippy::") {
                    push(l, t.to_string());
                    i += 1;
                    continue;
                }
            }
        }
        if let Some(l) = level_from_flag(a) {
            if let Some(n) = args.get(i + 1) {
                if n.starts_with("clippy::") {
                    push(l, n.clone());
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    m
}

#[allow(rustc::potential_query_instability)]
fn build_merged_clippy_lint_args(existing_args: &[String], overrides: &[(String, String, i64)]) -> Vec<String> {
    let mut all: Vec<(String, String, i64)> = parse_existing_lint_args(existing_args)
        .into_iter()
        .map(|(n, (l, p))| (n, l, p))
        .collect();

    for (name, level, prio) in overrides {
        all.retain(|(n, _, _)| n != name);
        all.push((name.clone(), level.clone(), *prio));
    }

    all.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
    let mut out = Vec::with_capacity(all.len() * 2);
    for (name, lvl, _) in all {
        let Some(flag) = flag_from_level(lvl.as_str()) else {
            continue;
        };
        out.push(flag.to_string());
        out.push(name);
    }
    out
}

pub fn apply_merged_clippy_lints(args: Vec<String>) -> Vec<String> {
    let overrides = read_clippy_toml_overrides();
    let merged = build_merged_clippy_lint_args(&args, &overrides);
    if merged.is_empty() {
        return args;
    }
    let mut out = Vec::with_capacity(args.len() + merged.len());
    let mut i = 0;
    let is_flag = |s: &str| level_from_flag(s).is_some();
    while i < args.len() {
        let a = &args[i];
        if let Some((f, v)) = a.split_once('=') {
            if is_flag(f) && v.starts_with("clippy::") {
                i += 1;
                continue;
            }
        }
        if a.len() > 2 {
            let (h, t) = a.split_at(2);
            if is_flag(h) && t.starts_with("clippy::") {
                i += 1;
                continue;
            }
        }
        if is_flag(a) {
            if let Some(n) = args.get(i + 1) {
                if n.starts_with("clippy::") {
                    i += 2;
                    continue;
                }
            }
        }
        out.push(a.clone());
        i += 1;
    }
    out.extend(merged);
    out
}

pub fn apply_merged_clippy_lints_from_path(args: Vec<String>, start_dir: &std::path::Path) -> Vec<String> {
    let overrides = read_clippy_toml_overrides_from(start_dir);
    let merged = build_merged_clippy_lint_args(&args, &overrides);
    if merged.is_empty() {
        return args;
    }
    let mut out = Vec::with_capacity(args.len() + merged.len());
    let mut i = 0;
    let is_flag = |s: &str| level_from_flag(s).is_some();
    while i < args.len() {
        let a = &args[i];
        if let Some((f, v)) = a.split_once('=') {
            if is_flag(f) && v.starts_with("clippy::") {
                i += 1;
                continue;
            }
        }
        if a.len() > 2 {
            let (h, t) = a.split_at(2);
            if is_flag(h) && t.starts_with("clippy::") {
                i += 1;
                continue;
            }
        }
        if is_flag(a) {
            if let Some(n) = args.get(i + 1) {
                if n.starts_with("clippy::") {
                    i += 2;
                    continue;
                }
            }
        }
        out.push(a.clone());
        i += 1;
    }
    out.extend(merged);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        apply_merged_clippy_lints, apply_merged_clippy_lints_from_path, build_merged_clippy_lint_args,
        parse_existing_lint_args,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn parse_examples() {
        let args = vec![
            "--allow=clippy::pedantic",
            "-Wclippy::redundant_clone",
            "-D",
            "clippy::unwrap_used",
            "--forbid",
            "clippy::dbg_macro",
            "--warn=unused_variables", // non-clippy → ignored
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();

        let m = parse_existing_lint_args(&args);
        assert_eq!(m.get("clippy::pedantic").map(|v| v.0.as_str()), Some("allow"));
        assert_eq!(m.get("clippy::redundant_clone").map(|v| v.0.as_str()), Some("warn"));
        assert_eq!(m.get("clippy::unwrap_used").map(|v| v.0.as_str()), Some("deny"));
        assert_eq!(m.get("clippy::dbg_macro").map(|v| v.0.as_str()), Some("forbid"));
        assert!(!m.contains_key("unused_variables"));
    }

    #[test]
    fn parse_ignores_unknown_and_non_clippy() {
        let args = vec![
            "--force-warn=clippy::all", // unknown level flag → ignored by parser
            "--allow",
            "not-a-lint",       // missing or non-clippy values → ignored
            "--warn=dead_code", // non-clippy lint → ignored
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();

        let m = parse_existing_lint_args(&args);
        assert!(m.is_empty());
    }

    #[test]
    fn build_merge_overrides_and_order() {
        // Existing args include various forms
        let existing = vec!["-W", "clippy::foo", "--deny=clippy::bar", "-Aclippy::baz"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>();

        // Overrides replace foo → deny (prio 0) and add new → warn (prio 5)
        let overrides = vec![
            ("clippy::foo".to_string(), "deny".to_string(), 0),
            ("clippy::new".to_string(), "warn".to_string(), 5),
        ];

        let merged = build_merged_clippy_lint_args(&existing, &overrides);

        // Convert flat args into pairs for assertions
        let pairs: Vec<(&str, &str)> = merged
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    Some((c[0].as_str(), c[1].as_str()))
                } else {
                    None
                }
            })
            .collect();

        // Expect replaced foo now as deny
        assert!(pairs.contains(&("-D", "clippy::foo")));

        // Existing bar deny retained, baz allow retained, and new warn added
        assert!(pairs.contains(&("-D", "clippy::bar")));
        assert!(pairs.contains(&("-A", "clippy::baz")));
        assert!(pairs.contains(&("-W", "clippy::new")));

        // Order: lowest priority first → foo (0) should appear before new (5)
        let idx_foo = pairs
            .iter()
            .position(|p| p == &("-D", "clippy::foo"))
            .expect("foo present");
        let idx_new = pairs
            .iter()
            .position(|p| p == &("-W", "clippy::new"))
            .expect("new present");
        assert!(idx_foo < idx_new);
    }

    #[test]
    fn merge_from_clippy_toml() {
        // Create a temporary config directory with a clippy.toml
        let base = std::env::temp_dir();
        let unique = format!(
            "clippy_merge_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        );
        let dir = base.join(unique);
        fs::create_dir_all(&dir).unwrap();

        let toml = r#"
[lints.clippy]
foo = "deny"
bar = { level = "warn", priority = 5 }
"#;
        fs::write(dir.join("clippy.toml"), toml).unwrap();

        // Existing args include an allow on foo which should be overridden by TOML to deny
        let existing = vec!["-A", "clippy::foo", "-Dclippy::baz"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>();

        let merged = apply_merged_clippy_lints_from_path(existing, Path::new(&dir));
        let pairs: Vec<(&str, &str)> = merged
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    Some((c[0].as_str(), c[1].as_str()))
                } else {
                    None
                }
            })
            .collect();

        // foo comes from TOML as deny, bar added as warn, baz retained as deny
        assert!(pairs.contains(&("-D", "clippy::foo")));
        assert!(pairs.contains(&("-W", "clippy::bar")));
        assert!(pairs.contains(&("-D", "clippy::baz")));

        // Cleanup files
        let _ = fs::remove_file(dir.join("clippy.toml"));
        let _ = fs::remove_dir(&dir);
    }
}
