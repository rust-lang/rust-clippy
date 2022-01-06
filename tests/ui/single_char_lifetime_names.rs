#![warn(clippy::single_char_lifetime_names)]

struct DiagnosticCtx<'a, 'b>
where
    'a: 'b,
{
    _source: &'a str,
    _unit: &'b (),
}

impl<'a, 'b> DiagnosticCtx<'a, 'b> {
    fn new(source: &'a str, unit: &'b ()) -> DiagnosticCtx<'a, 'b> {
        Self {
            _source: source,
            _unit: unit,
        }
    }
}

impl<'src, 'unit> DiagnosticCtx<'src, 'unit> {
    fn new_pass(source: &'src str, unit: &'unit ()) -> DiagnosticCtx<'src, 'unit> {
        Self {
            _source: source,
            _unit: unit,
        }
    }
}

fn main() {
    let src = "loop {}";
    let unit = ();
    DiagnosticCtx::new(src, &unit);
}
