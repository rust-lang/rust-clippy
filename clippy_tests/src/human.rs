#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CargoMessage {
  message: Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Diagnostic {
  rendered: String,
}

pub fn ui_from_json(input: &str) -> String {
    input.lines()
        .map(|l| ::serde_json::from_str::<CargoMessage>(l))
        // eat parsing errors
        .flat_map(|line| line.ok())
        // One diagnostic line might have multiple suggestions
        .map(|c| c.message.rendered)
        .collect()
}
