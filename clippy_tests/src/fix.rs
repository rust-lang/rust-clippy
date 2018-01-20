use rustfix::{Suggestion, Replacement};
use failure::Error;

pub fn apply_suggestions(file_content: &str, suggestions: &[Suggestion]) -> Result<String, Error> {
    let mut fixed = String::from(file_content);

    for sug in suggestions.into_iter().rev() {
        trace!("{:?}", sug);
        for sol in &sug.solutions {
            trace!("{:?}", sol);
            for r in &sol.replacements {
                debug!("replaced.");
                trace!("{:?}", r);
                fixed = apply_suggestion(&mut fixed, r)?;
            }
        }
    }

    Ok(fixed)
}

fn apply_suggestion(file_content: &mut String, suggestion: &Replacement) -> Result<String, Error> {
    use std::cmp::max;

    let mut new_content = String::new();

    // Add the lines before the section we want to replace
    new_content.push_str(&file_content.lines()
        .take(max(suggestion.snippet.line_range.start.line - 1, 0) as usize)
        .collect::<Vec<_>>()
        .join("\n"));
    new_content.push_str("\n");

    // Parts of line before replacement
    new_content.push_str(&file_content.lines()
        .nth(suggestion.snippet.line_range.start.line - 1)
        .unwrap_or("")
        .chars()
        .take(suggestion.snippet.line_range.start.column - 1)
        .collect::<String>());

    // Insert new content! Finally!
    new_content.push_str(&suggestion.replacement);

    // Parts of line after replacement
    new_content.push_str(&file_content.lines()
        .nth(suggestion.snippet.line_range.end.line - 1)
        .unwrap_or("")
        .chars()
        .skip(suggestion.snippet.line_range.end.column - 1)
        .collect::<String>());

    // Add the lines after the section we want to replace
    new_content.push_str("\n");
    new_content.push_str(&file_content.lines()
        .skip(suggestion.snippet.line_range.end.line as usize)
        .collect::<Vec<_>>()
        .join("\n"));
    new_content.push_str("\n");

    Ok(new_content)
}
