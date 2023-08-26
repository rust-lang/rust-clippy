use cargo_metadata::Metadata;
use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_lint::LateContext;
use rustc_span::source_map::DUMMY_SP;

use super::REQUIRE_WORKSPACE_DEPENDENCIES;

pub(super) fn check(cx: &LateContext<'_>, metadata: &Metadata) {
    let manifest_path = &metadata.packages[0].manifest_path;

    let Ok(manifest) = std::fs::read_to_string(manifest_path) else {
        span_lint(
            cx,
            REQUIRE_WORKSPACE_DEPENDENCIES,
            DUMMY_SP,
            &format!("unable to read the crate manifest `{manifest_path}`"),
        );
        return;
    };
    let Ok(manifest) = toml::from_str::<Manifest>(&manifest) else {
        span_lint(
            cx,
            REQUIRE_WORKSPACE_DEPENDENCIES,
            DUMMY_SP,
            &format!("unable to parse the crate manifest `{manifest_path}`"),
        );
        return;
    };

    let all_deps = [
        manifest.dependencies,
        manifest.dev_dependencies,
        manifest.build_dependencies,
    ]
    .into_iter()
    .flat_map(std::iter::IntoIterator::into_iter)
    .flat_map(std::iter::IntoIterator::into_iter);

    for (name, dep) in all_deps {
        if_chain! {
            if let Some(workspace) = dep.get("workspace");
            if let Some(is_workspace_dep) = workspace.as_bool();
            if is_workspace_dep;
            then {
                continue;
            }
        }

        span_lint(
            cx,
            REQUIRE_WORKSPACE_DEPENDENCIES,
            DUMMY_SP,
            &format!("non-workspace dependency `{name}`"),
        );
    }
}

/// The bare-bones [`Cargo.toml`] manifest to parse out the dependencies.
#[derive(Debug, serde::Deserialize)]
#[serde(rename = "param-case")]
pub struct Manifest {
    pub dependencies: Option<toml::Table>,
    pub dev_dependencies: Option<toml::Table>,
    pub build_dependencies: Option<toml::Table>,
}
