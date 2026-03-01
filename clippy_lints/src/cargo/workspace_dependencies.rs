use clippy_utils::diagnostics::span_lint;
use rustc_data_structures::fx::FxHashSet;
use rustc_lint::LateContext;
use rustc_span::{BytePos, Pos, SourceFile, Span, SyntaxContext, DUMMY_SP};
use std::ops::Range;
use std::path::Path;
use toml::de::{DeTable, DeValue};

use super::WORKSPACE_DEPENDENCIES;

fn toml_span(range: Range<usize>, file: &SourceFile) -> Span {
    Span::new(
        file.start_pos + BytePos::from_usize(range.start),
        file.start_pos + BytePos::from_usize(range.end),
        SyntaxContext::root(),
        None,
    )
}

fn is_workspace_dep(value: &DeValue<'_>) -> bool {
    match value {
        DeValue::Table(tbl) => {
            if let Some(workspace) = tbl.get("workspace") {
                if let DeValue::Boolean(b) = workspace.get_ref() {
                    return *b;
                }
            }
            false
        }
        _ => false,
    }
}

fn has_inline_version_info(value: &DeValue<'_>) -> bool {
    match value {
        DeValue::String(_) => true, // e.g., serde = "1.0"
        DeValue::Table(tbl) => {
            // Check if it has version, git, or path fields (but not workspace = true)
            if is_workspace_dep(value) {
                return false;
            }
            tbl.contains_key("version") || tbl.contains_key("git") || tbl.contains_key("path")
        }
        _ => false,
    }
}

fn get_workspace_deps(cargo_toml: &DeTable<'_>) -> FxHashSet<String> {
    let mut workspace_deps = FxHashSet::default();

    if let Some(workspace) = cargo_toml.get("workspace")
        && let Some(workspace_tbl) = workspace.get_ref().as_table()
        && let Some(dependencies) = workspace_tbl.get("dependencies")
        && let Some(deps_tbl) = dependencies.get_ref().as_table()
    {
        for dep_name in deps_tbl.keys() {
            workspace_deps.insert(dep_name.get_ref().to_string());
        }
    }

    workspace_deps
}

fn check_dependencies(
    cx: &LateContext<'_>,
    deps_tbl: &DeTable<'_>,
    workspace_deps: &FxHashSet<String>,
    _file: &SourceFile,
    section_name: &str,
) {
    for (dep_name, dep_value) in deps_tbl {
        let name = dep_name.get_ref().as_ref();

        if workspace_deps.contains(name) && has_inline_version_info(dep_value.get_ref()) {
            span_lint(
                cx,
                WORKSPACE_DEPENDENCIES,
                DUMMY_SP,
                format!("dependency `{name}` is defined in workspace but not using `workspace = true` in {section_name}"),
            );
        }
    }
}

pub fn check(cx: &LateContext<'_>) {
    if let Ok(file) = cx.tcx.sess.source_map().load_file(Path::new("Cargo.toml"))
        && let Some(src) = file.src.as_deref()
        && let Ok(cargo_toml) = DeTable::parse(src)
    {
        // First, collect all workspace dependencies
        let workspace_deps = get_workspace_deps(cargo_toml.get_ref());

        // If there are no workspace dependencies, nothing to check
        if workspace_deps.is_empty() {
            return;
        }

        // Check [dependencies]
        if let Some(dependencies) = cargo_toml.get_ref().get("dependencies")
            && let Some(deps_tbl) = dependencies.get_ref().as_table()
        {
            check_dependencies(cx, deps_tbl, &workspace_deps, &file, "[dependencies]");
        }

        // Check [dev-dependencies]
        if let Some(dev_dependencies) = cargo_toml.get_ref().get("dev-dependencies")
            && let Some(dev_deps_tbl) = dev_dependencies.get_ref().as_table()
        {
            check_dependencies(cx, dev_deps_tbl, &workspace_deps, &file, "[dev-dependencies]");
        }

        // Check [build-dependencies]
        if let Some(build_dependencies) = cargo_toml.get_ref().get("build-dependencies")
            && let Some(build_deps_tbl) = build_dependencies.get_ref().as_table()
        {
            check_dependencies(cx, build_deps_tbl, &workspace_deps, &file, "[build-dependencies]");
        }
    }
}
