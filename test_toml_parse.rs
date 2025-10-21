use toml::de::DeTable;

fn main() {
    let content = r#"
[package]
name = "workspace_dependencies"
version = "0.1.0"
publish = false

[workspace]
members = []

[workspace.dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["rt"] }
"#;

    match DeTable::parse(content) {
        Ok(cargo_toml) => {
            println!("Parsed successfully!");

            // Check workspace.dependencies
            if let Some(workspace) = cargo_toml.get_ref().get("workspace") {
                println!("Found workspace");
                if let Some(workspace_tbl) = workspace.get_ref().as_table() {
                    if let Some(deps) = workspace_tbl.get("dependencies") {
                        println!("Found workspace.dependencies");
                        if let Some(deps_tbl) = deps.get_ref().as_table() {
                            println!("Workspace dependencies:");
                            for (name, _) in deps_tbl {
                                println!("  - {}", name.get_ref());
                            }
                        }
                    }
                }
            }

            // Check dependencies
            if let Some(deps) = cargo_toml.get_ref().get("dependencies") {
                println!("Found dependencies");
                if let Some(deps_tbl) = deps.get_ref().as_table() {
                    println!("Package dependencies:");
                    for (name, value) in deps_tbl {
                        println!("  - {} = {:?}", name.get_ref(), value.get_ref());
                    }
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}
