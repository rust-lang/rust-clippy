use crate::types::{CargoToml, Lints};
use rustc_session::Session;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug)]
pub struct MergedLintConfig {
    pub rust_lints: BTreeMap<String, (String, i64, Option<String>)>, // (level, priority, source)
    pub clippy_lints: BTreeMap<String, (String, i64, Option<String>)>, // (level, priority, source)
}

impl MergedLintConfig {
    pub fn load(sess: &Session) -> Self {
        let mut merged = MergedLintConfig {
            rust_lints: BTreeMap::new(),
            clippy_lints: BTreeMap::new(),
        };

        // Load from Cargo.toml
        if let Ok(file) = sess.source_map().load_file(Path::new("Cargo.toml"))
            && let Some(src) = file.src.as_deref()
            && let Ok(cargo_toml) = toml::from_str::<CargoToml>(src)
        {
            merged.merge_cargo_config(&cargo_toml, sess);
        }

        // Load from clippy.toml
        if let Ok((Some(clippy_config_path), _)) = crate::lookup_conf_file()
            && let Ok(file) = sess.source_map().load_file(&clippy_config_path)
            && let Some(src) = file.src.as_deref()
        {
            // Try parsing as a full CargoToml structure (with [lints] sections)
            if let Ok(clippy_config) = toml::from_str::<CargoToml>(src) {
                merged.merge_lints(&clippy_config.lints, "clippy.toml", sess);
                merged.merge_lints(&clippy_config.workspace.lints, "clippy.toml [workspace]", sess);
            } else if let Ok(clippy_lints) = toml::from_str::<Lints>(src) {
                // Fallback: try parsing as just the lints section
                merged.merge_lints(&clippy_lints, "clippy.toml", sess);
            }
        }

        merged
    }

    pub fn load_static() -> Self {
        let mut merged = MergedLintConfig {
            rust_lints: BTreeMap::new(),
            clippy_lints: BTreeMap::new(),
        };

        // Load from Cargo.toml
        if let Ok(src) = std::fs::read_to_string("Cargo.toml")
            && let Ok(cargo_toml) = toml::from_str::<CargoToml>(&src)
        {
            merged.merge_cargo_config_static(&cargo_toml);
        }

        // Load from clippy.toml
        if let Ok((Some(clippy_config_path), _)) = crate::lookup_conf_file()
            && let Ok(src) = std::fs::read_to_string(&clippy_config_path)
        {
            // Try parsing as a full CargoToml structure (with [lints] sections)
            if let Ok(clippy_config) = toml::from_str::<CargoToml>(&src) {
                merged.merge_lints_static(&clippy_config.lints, "clippy.toml");
                merged.merge_lints_static(&clippy_config.workspace.lints, "clippy.toml [workspace]");
            } else if let Ok(clippy_lints) = toml::from_str::<Lints>(&src) {
                // Fallback: try parsing as just the lints section
                merged.merge_lints_static(&clippy_lints, "clippy.toml");
            }
        }
        merged
    }

    /// Create a `MergedLintConfig` from TOML strings (for testing)
    ///
    /// `cargo_toml` should be a full Cargo.toml with [lints.clippy] and [lints.rust] sections
    /// `clippy_toml` should be in the format expected by clippy.toml with [lints.clippy] and
    /// [lints.rust] sections
    pub fn from_toml_strings(cargo_toml: Option<&str>, clippy_toml: Option<&str>) -> Self {
        let mut merged = MergedLintConfig {
            rust_lints: BTreeMap::new(),
            clippy_lints: BTreeMap::new(),
        };

        // Parse Cargo.toml if provided
        if let Some(cargo_src) = cargo_toml
            && let Ok(cargo_config) = toml::from_str::<CargoToml>(cargo_src)
        {
            merged.merge_cargo_config_static(&cargo_config);
        }

        // Parse clippy.toml if provided - it has the same structure as Cargo.toml [lints] sections
        if let Some(clippy_src) = clippy_toml {
            // Try parsing as a full CargoToml structure (with [lints] sections)
            if let Ok(clippy_config) = toml::from_str::<CargoToml>(clippy_src) {
                merged.merge_lints_static(&clippy_config.lints, "clippy.toml");
                merged.merge_lints_static(&clippy_config.workspace.lints, "clippy.toml [workspace]");
            } else if let Ok(clippy_config) = toml::from_str::<Lints>(clippy_src) {
                // Fallback: try parsing as just the lints section
                merged.merge_lints_static(&clippy_config, "clippy.toml");
            }
        }

        merged
    }

    fn merge_cargo_config(&mut self, cargo_toml: &CargoToml, sess: &Session) {
        self.merge_lints(&cargo_toml.lints, "Cargo.toml", sess);
        self.merge_lints(&cargo_toml.workspace.lints, "Cargo.toml [workspace]", sess);
    }

    fn merge_cargo_config_static(&mut self, cargo_toml: &CargoToml) {
        self.merge_lints_static(&cargo_toml.lints, "Cargo.toml");
        self.merge_lints_static(&cargo_toml.workspace.lints, "Cargo.toml [workspace]");
    }

    fn merge_lints(&mut self, lints: &Lints, source: &str, sess: &Session) {
        // Merge rust lints
        for (name, config) in &lints.rust {
            let name_str = name.get_ref().clone();
            let level = config.get_ref().level().to_string();
            let priority = config.get_ref().priority();

            if let Some((existing_level, existing_priority, existing_source)) = self.rust_lints.get(&name_str) {
                // Only warn for conflicts between different file types (Cargo.toml vs clippy.toml)
                let existing_is_cargo = existing_source.as_deref().unwrap_or("").contains("Cargo.toml");
                let current_is_cargo = source.contains("Cargo.toml");
                if existing_is_cargo != current_is_cargo && (existing_level != &level || existing_priority != &priority)
                {
                    sess.dcx().warn(format!(
                        "Conflicting configuration for rust lint '{}': {}@{} in {} vs {}@{} in {}",
                        name_str,
                        existing_level,
                        existing_priority,
                        existing_source.as_deref().unwrap_or("unknown"),
                        level,
                        priority,
                        source
                    ));
                }
                // clippy.toml takes precedence over Cargo.toml
                if source == "clippy.toml" {
                    self.rust_lints
                        .insert(name_str, (level, priority, Some(source.to_string())));
                }
            } else {
                self.rust_lints
                    .insert(name_str, (level, priority, Some(source.to_string())));
            }
        }

        // Merge clippy lints
        for (name, config) in &lints.clippy {
            let name_str = name.get_ref().clone();
            let level = config.get_ref().level().to_string();
            let priority = config.get_ref().priority();

            if let Some((existing_level, existing_priority, existing_source)) = self.clippy_lints.get(&name_str) {
                // Only warn for conflicts between different file types (Cargo.toml vs clippy.toml)
                let existing_is_cargo = existing_source.as_deref().unwrap_or("").contains("Cargo.toml");
                let current_is_cargo = source.contains("Cargo.toml");
                if existing_is_cargo != current_is_cargo && (existing_level != &level || existing_priority != &priority)
                {
                    sess.dcx().warn(format!(
                        "Conflicting configuration for clippy lint '{}': {}@{} in {} vs {}@{} in {}",
                        name_str,
                        existing_level,
                        existing_priority,
                        existing_source.as_deref().unwrap_or("unknown"),
                        level,
                        priority,
                        source
                    ));
                }
                // clippy.toml takes precedence over Cargo.toml
                if source == "clippy.toml" {
                    self.clippy_lints
                        .insert(name_str, (level, priority, Some(source.to_string())));
                }
            } else {
                self.clippy_lints
                    .insert(name_str, (level, priority, Some(source.to_string())));
            }
        }
    }

    fn merge_lints_static(&mut self, lints: &Lints, source: &str) {
        // Merge rust lints
        for (name, config) in &lints.rust {
            let name_str = name.get_ref().clone();
            let level = config.get_ref().level().to_string();
            let priority = config.get_ref().priority();

            if let Some((existing_level, existing_priority, existing_source)) = self.rust_lints.get(&name_str) {
                // Only warn for conflicts between different file types (Cargo.toml vs clippy.toml)
                let existing_is_cargo = existing_source.as_deref().unwrap_or("").contains("Cargo.toml");
                let current_is_cargo = source.contains("Cargo.toml");
                if existing_is_cargo != current_is_cargo && (existing_level != &level || existing_priority != &priority)
                {
                    eprintln!(
                        "Warning: Conflicting configuration for rust lint '{}': {}@{} in {} vs {}@{} in {}",
                        name_str,
                        existing_level,
                        existing_priority,
                        existing_source.as_deref().unwrap_or("unknown"),
                        level,
                        priority,
                        source
                    );
                }
                // clippy.toml takes precedence over Cargo.toml
                if source == "clippy.toml" {
                    self.rust_lints
                        .insert(name_str, (level, priority, Some(source.to_string())));
                }
            } else {
                self.rust_lints
                    .insert(name_str, (level, priority, Some(source.to_string())));
            }
        }

        // Merge clippy lints
        for (name, config) in &lints.clippy {
            let name_str = name.get_ref().clone();
            let level = config.get_ref().level().to_string();
            let priority = config.get_ref().priority();

            if let Some((existing_level, existing_priority, existing_source)) = self.clippy_lints.get(&name_str) {
                // Only warn for conflicts between different file types (Cargo.toml vs clippy.toml)
                let existing_is_cargo = existing_source.as_deref().unwrap_or("").contains("Cargo.toml");
                let current_is_cargo = source.contains("Cargo.toml");
                if existing_is_cargo != current_is_cargo && (existing_level != &level || existing_priority != &priority)
                {
                    eprintln!(
                        "Warning: Conflicting configuration for clippy lint '{}': {}@{} in {} vs {}@{} in {}",
                        name_str,
                        existing_level,
                        existing_priority,
                        existing_source.as_deref().unwrap_or("unknown"),
                        level,
                        priority,
                        source
                    );
                }
                // clippy.toml takes precedence over Cargo.toml
                if source == "clippy.toml" {
                    self.clippy_lints
                        .insert(name_str, (level, priority, Some(source.to_string())));
                }
            } else {
                self.clippy_lints
                    .insert(name_str, (level, priority, Some(source.to_string())));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_toml_only() {
        let cargo_toml = r#"
[lints.clippy]
needless_return = "allow"
single_match = { level = "warn", priority = 5 }

[lints.rust]
dead_code = "allow"
unused_variables = { level = "warn", priority = 10 }
"#;

        let config = MergedLintConfig::from_toml_strings(Some(cargo_toml), None);

        // Check clippy lints
        assert_eq!(config.clippy_lints.len(), 2);
        assert_eq!(
            config.clippy_lints["needless_return"],
            ("allow".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["single_match"],
            ("warn".to_string(), 5, Some("Cargo.toml".to_string()))
        );

        // Check rust lints
        assert_eq!(config.rust_lints.len(), 2);
        assert_eq!(
            config.rust_lints["dead_code"],
            ("allow".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_variables"],
            ("warn".to_string(), 10, Some("Cargo.toml".to_string()))
        );
    }

    #[test]
    fn test_clippy_toml_only() {
        let clippy_toml = r#"
[lints.clippy]
needless_return = "deny"
too_many_arguments = { level = "forbid", priority = 15 }

[lints.rust]
dead_code = { level = "warn", priority = 3 }
unused_imports = "allow"
"#;

        let config = MergedLintConfig::from_toml_strings(None, Some(clippy_toml));

        // Check clippy lints
        assert_eq!(config.clippy_lints.len(), 2);
        assert_eq!(
            config.clippy_lints["needless_return"],
            ("deny".to_string(), 0, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["too_many_arguments"],
            ("forbid".to_string(), 15, Some("clippy.toml".to_string()))
        );

        // Check rust lints
        assert_eq!(config.rust_lints.len(), 2);
        assert_eq!(
            config.rust_lints["dead_code"],
            ("warn".to_string(), 3, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_imports"],
            ("allow".to_string(), 0, Some("clippy.toml".to_string()))
        );
    }

    #[test]
    fn test_merged_configs_no_conflicts() {
        let cargo_toml = r#"
[lints.clippy]
needless_return = "allow"
single_match = { level = "warn", priority = 5 }

[lints.rust]
dead_code = "allow"
unused_variables = { level = "warn", priority = 10 }
"#;

        let clippy_toml = r#"
[lints.clippy]
too_many_arguments = { level = "forbid", priority = 15 }
wildcard_imports = "deny"

[lints.rust]
unused_imports = "allow"
unreachable_code = { level = "warn", priority = 8 }
"#;

        let config = MergedLintConfig::from_toml_strings(Some(cargo_toml), Some(clippy_toml));

        // Check clippy lints (should have lints from both files)
        assert_eq!(config.clippy_lints.len(), 4);
        assert_eq!(
            config.clippy_lints["needless_return"],
            ("allow".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["single_match"],
            ("warn".to_string(), 5, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["too_many_arguments"],
            ("forbid".to_string(), 15, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["wildcard_imports"],
            ("deny".to_string(), 0, Some("clippy.toml".to_string()))
        );

        // Check rust lints (should have lints from both files)
        assert_eq!(config.rust_lints.len(), 4);
        assert_eq!(
            config.rust_lints["dead_code"],
            ("allow".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_variables"],
            ("warn".to_string(), 10, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_imports"],
            ("allow".to_string(), 0, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unreachable_code"],
            ("warn".to_string(), 8, Some("clippy.toml".to_string()))
        );
    }

    #[test]
    fn test_clippy_toml_precedence() {
        let cargo_toml = r#"
[lints.clippy]
needless_return = "allow"
single_match = { level = "warn", priority = 5 }

[lints.rust]
dead_code = "allow"
unused_variables = { level = "warn", priority = 10 }
"#;

        let clippy_toml = r#"
[lints.clippy]
needless_return = "deny"
single_match = { level = "forbid", priority = 15 }

[lints.rust]
dead_code = { level = "warn", priority = 3 }
unused_variables = "forbid"
"#;

        let config = MergedLintConfig::from_toml_strings(Some(cargo_toml), Some(clippy_toml));

        // Check that clippy.toml values take precedence
        assert_eq!(config.clippy_lints.len(), 2);
        assert_eq!(
            config.clippy_lints["needless_return"],
            ("deny".to_string(), 0, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["single_match"],
            ("forbid".to_string(), 15, Some("clippy.toml".to_string()))
        );

        assert_eq!(config.rust_lints.len(), 2);
        assert_eq!(
            config.rust_lints["dead_code"],
            ("warn".to_string(), 3, Some("clippy.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_variables"],
            ("forbid".to_string(), 0, Some("clippy.toml".to_string()))
        );
    }

    #[test]
    fn test_workspace_lints() {
        let cargo_toml = r#"
[lints.clippy]
needless_return = "allow"

[lints.rust]
dead_code = "warn"

[workspace.lints.clippy]
single_match = { level = "deny", priority = 20 }

[workspace.lints.rust]
unused_variables = "forbid"
"#;

        let config = MergedLintConfig::from_toml_strings(Some(cargo_toml), None);

        // Check that both regular and workspace lints are included
        assert_eq!(config.clippy_lints.len(), 2);
        assert_eq!(
            config.clippy_lints["needless_return"],
            ("allow".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.clippy_lints["single_match"],
            ("deny".to_string(), 20, Some("Cargo.toml [workspace]".to_string()))
        );

        assert_eq!(config.rust_lints.len(), 2);
        assert_eq!(
            config.rust_lints["dead_code"],
            ("warn".to_string(), 0, Some("Cargo.toml".to_string()))
        );
        assert_eq!(
            config.rust_lints["unused_variables"],
            ("forbid".to_string(), 0, Some("Cargo.toml [workspace]".to_string()))
        );
    }

    #[test]
    fn test_priority_parsing() {
        let cargo_toml = r#"
[lints.clippy]
needless_return = "allow"
single_match = { level = "warn", priority = 5 }
too_many_arguments = { level = "deny", priority = -10 }
wildcard_imports = { level = "forbid" }
"#;

        let config = MergedLintConfig::from_toml_strings(Some(cargo_toml), None);

        assert_eq!(config.clippy_lints.len(), 4);
        assert_eq!(config.clippy_lints["needless_return"].1, 0); // Default priority
        assert_eq!(config.clippy_lints["single_match"].1, 5);
        assert_eq!(config.clippy_lints["too_many_arguments"].1, -10); // Negative priority
        assert_eq!(config.clippy_lints["wildcard_imports"].1, 0); // Missing priority defaults to 0
    }

    #[test]
    fn test_empty_configs() {
        let config = MergedLintConfig::from_toml_strings(None, None);
        assert_eq!(config.clippy_lints.len(), 0);
        assert_eq!(config.rust_lints.len(), 0);

        let empty_cargo = r#"
[package]
name = "test"
version = "0.1.0"
"#;

        let config = MergedLintConfig::from_toml_strings(Some(empty_cargo), None);
        assert_eq!(config.clippy_lints.len(), 0);
        assert_eq!(config.rust_lints.len(), 0);
    }

    #[test]
    fn test_malformed_toml_ignored() {
        let malformed_cargo = r#"
[lints.clippy
needless_return = "allow"
"#;

        let valid_clippy = r#"
[lints.clippy]
single_match = "warn"
"#;

        let config = MergedLintConfig::from_toml_strings(Some(malformed_cargo), Some(valid_clippy));

        // Should only have the valid clippy.toml content
        assert_eq!(config.clippy_lints.len(), 1);
        assert_eq!(
            config.clippy_lints["single_match"],
            ("warn".to_string(), 0, Some("clippy.toml".to_string()))
        );
        assert_eq!(config.rust_lints.len(), 0);
    }
}
