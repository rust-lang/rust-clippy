#![allow(dead_code)]

#[warn(clippy::invisible_characters)]
fn zero() {
    print!("Here >​< is a ZWS, and ​another");
    //~^ ERROR: invisible character detected
    //~| NOTE: `-D clippy::invisible-characters` implied by `-D warnings`
    print!("This\u{200B}is\u{200B}fine");
    print!("Here >­< is a SHY, and ­another");
    //~^ ERROR: invisible character detected
    print!("This\u{ad}is\u{ad}fine");
    print!("Here >⁠< is a WJ, and ⁠another");
    //~^ ERROR: invisible character detected
    print!("This\u{2060}is\u{2060}fine");
}

#[warn(clippy::unicode_not_nfc)]
fn canon() {
    print!("̀àh?");
    //~^ ERROR: non-NFC Unicode sequence detected
    //~| NOTE: `-D clippy::unicode-not-nfc` implied by `-D warnings`
    print!("a\u{0300}h?"); // also ok
}

mod non_ascii_literal {
    #![deny(clippy::non_ascii_literal)]

    fn uni() {
        print!("Üben!");
        //~^ ERROR: literal non-ASCII character detected
        print!("\u{DC}ben!"); // this is ok
    }

    // issue 8013
    fn single_quote() {
        const _EMPTY_BLOCK: char = '▱';
        //~^ ERROR: literal non-ASCII character detected
        const _FULL_BLOCK: char = '▰';
        //~^ ERROR: literal non-ASCII character detected
    }

    #[test]
    pub fn issue_7739() {
        // Ryū crate: https://github.com/dtolnay/ryu
    }

    mod issue_8263 {
        #![deny(clippy::non_ascii_literal)]

        // Re-allow for a single test
        #[test]
        #[allow(clippy::non_ascii_literal)]
        fn allowed() {
            let _ = "悲しいかな、ここに日本語を書くことはできない。";
        }

        #[test]
        fn denied() {
            let _ = "悲しいかな、ここに日本語を書くことはできない。";
            //~^ ERROR: literal non-ASCII character detected
        }
    }
}

fn main() {
    zero();
    canon();
}
