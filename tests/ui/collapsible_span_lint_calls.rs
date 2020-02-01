[warn(clippy::collapsible_span_lint_calls)]

fn lint_something() {
    let msg = "warn about something";
    let help = "this would help";
    let note= "specificly note on this thing";

    let span = ?;
    let lint = Lint.default_fields_for_macro();
    let cx = ?;

    let sugg = "suggest changes";
    let applicability = Applicability::MachineApllicable;

    span_lint_and_then(cx, lint, span, msg, |db| {
        db.span_help(span, help);
    });

    span_lint_and_then(cx, lint, span, msg, |db| {
        db.span_note(span, note);
    });

    span_lint_and_then(cx, lint, span, msg, |db| {
        db.span_suggestion(span, help, sugg, applicability);
    });
}
