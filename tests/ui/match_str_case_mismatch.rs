#![warn(clippy::match_str_case_mismatch)]
#![allow(dead_code)]

// Valid

fn as_str_match() {
    let var = "BAR";

    match var.to_ascii_lowercase().as_str() {
        "foo" => {},
        "bar" => {},
        _ => {},
    }
}

fn non_alphabetic() {
    let var = "~!@#$%^&*()-_=+FOO";

    match var.to_ascii_lowercase().as_str() {
        "1234567890" => {},
        "~!@#$%^&*()-_=+foo" => {},
        "\n\r\t\x7F" => {},
        _ => {},
    }
}

fn unicode_cased() {
    let var = "ВОДЫ";

    match var.to_lowercase().as_str() {
        "水" => {},
        "νερό" => {},
        "воды" => {},
        "물" => {},
        _ => {},
    }
}

fn titlecase() {
    let var = "Barǲ";

    match var.to_lowercase().as_str() {
        "fooǉ" => {},
        "barǳ" => {},
        _ => {},
    }
}

fn no_case_equivalent() {
    let var = "barʁ";

    match var.to_uppercase().as_str() {
        "FOOɕ" => {},
        "BARʁ" => {},
        _ => {},
    }
}

fn addrof_unary_match() {
    let var = "BAR";

    match &*var.to_ascii_lowercase() {
        "foo" => {},
        "bar" => {},
        _ => {},
    }
}

fn alternating_chain() {
    let var = "BAR";

    match &*var
        .to_ascii_lowercase()
        .to_uppercase()
        .to_lowercase()
        .to_ascii_uppercase()
    {
        "FOO" => {},
        "BAR" => {},
        _ => {},
    }
}

fn unrelated_method() {
    struct Item {
        a: String,
    }

    impl Item {
        #[allow(clippy::wrong_self_convention)]
        fn to_lowercase(self) -> String {
            self.a
        }
    }

    let item = Item { a: String::from("BAR") };

    match &*item.to_lowercase() {
        "FOO" => {},
        "BAR" => {},
        _ => {},
    }
}

// Invalid

fn as_str_match_mismatch() {
    let var = "BAR";

    match var.to_ascii_lowercase().as_str() {
        "foo" => {},
        "Bar" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        //~| NOTE: `-D clippy::match-str-case-mismatch` implied by `-D warnings`
        _ => {},
    }
}

fn non_alphabetic_mismatch() {
    let var = "~!@#$%^&*()-_=+FOO";

    match var.to_ascii_lowercase().as_str() {
        "1234567890" => {},
        "~!@#$%^&*()-_=+Foo" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        "\n\r\t\x7F" => {},
        _ => {},
    }
}

fn unicode_cased_mismatch() {
    let var = "ВОДЫ";

    match var.to_lowercase().as_str() {
        "水" => {},
        "νερό" => {},
        "Воды" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        "물" => {},
        _ => {},
    }
}

fn titlecase_mismatch() {
    let var = "Barǲ";

    match var.to_lowercase().as_str() {
        "fooǉ" => {},
        "barǲ" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        _ => {},
    }
}

fn no_case_equivalent_mismatch() {
    let var = "barʁ";

    match var.to_uppercase().as_str() {
        "FOOɕ" => {},
        "bARʁ" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        _ => {},
    }
}

fn addrof_unary_match_mismatch() {
    let var = "BAR";

    match &*var.to_ascii_lowercase() {
        "foo" => {},
        "Bar" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        _ => {},
    }
}

fn alternating_chain_mismatch() {
    let var = "BAR";

    match &*var
        .to_ascii_lowercase()
        .to_uppercase()
        .to_lowercase()
        .to_ascii_uppercase()
    {
        "FOO" => {},
        "bAR" => {},
        //~^ ERROR: this `match` arm has a differing case than its expression
        _ => {},
    }
}

fn main() {}
