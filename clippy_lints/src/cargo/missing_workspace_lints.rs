use super::MISSING_WORKSPACE_LINTS;
use cargo_metadata::Metadata;
use clippy_utils::diagnostics::span_lint;
use rustc_lint::LateContext;
use rustc_span::DUMMY_SP;
use serde::Deserialize;
use std::path::Path;

type Lints = toml::Table;

#[derive(Deserialize, Debug, Default)]
struct Workspace {
    #[serde(default)]
    lints: Lints,
}

#[derive(Deserialize, Debug)]
struct CargoToml {
    #[serde(default)]
    lints: Lints,
    #[serde(default)]
    workspace: Workspace,
}

pub fn check(cx: &LateContext<'_>, metadata: &Metadata) {
    if let Ok(file) = cx.tcx.sess.source_map().load_file(Path::new("Cargo.toml"))
        && let Some(src) = file.src.as_deref()
        && let Ok(cargo_toml) = toml::from_str::<CargoToml>(src)
        // if `[workspace.lints]` exists,
        && !cargo_toml.workspace.lints.is_empty()
    {
        // for each project that is included in the workspace,
        for package in &metadata.packages {
            // if the project's Cargo.toml doesn't have lints.workspace = true
            if let Ok(file) = cx.tcx.sess.source_map().load_file(package.manifest_path.as_std_path())
                && let Some(src) = file.src.as_deref()
                && let Ok(cargo_toml) = toml::from_str::<CargoToml>(src)
                && !cargo_toml.lints.contains_key("workspace")
            {
                // TODO: Make real span
                span_lint(
                    cx,
                    MISSING_WORKSPACE_LINTS,
                    DUMMY_SP,
                    format!(
                        "Your project {} is in a workspace with lints configured, but workspace.lints is not configured.",
                        package.name
                    ),
                );
            }
        }
    }
}
