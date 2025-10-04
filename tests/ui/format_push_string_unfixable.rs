//@no-rustfix
#![warn(clippy::format_push_string)]

mod issue9493 {
    pub fn u8vec_to_hex(vector: &Vec<u8>, upper: bool) -> String {
        let mut hex = String::with_capacity(vector.len() * 2);
        for byte in vector {
            hex += &(if upper {
                format!("{byte:02X}")
                //~^ format_push_string
            } else {
                format!("{byte:02x}")
            });
        }
        hex
    }

    pub fn other_cases() {
        let mut s = String::new();
        // if let
        s += &(if let Some(_a) = Some(1234) {
            format!("{}", 1234)
            //~^ format_push_string
        } else {
            format!("{}", 1234)
        });
        // match
        s += &(match Some(1234) {
            Some(_) => format!("{}", 1234),
            //~^ format_push_string
            None => format!("{}", 1234),
        });
    }
}

fn main() {}
