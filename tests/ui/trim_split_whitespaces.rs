// run-rustfix
#![warn(clippy::trim_split_whitespaces)]
#![allow(clippy::let_unit_value)]

struct Custom;
impl Custom {
    fn trim(self) -> Self {
        self
    }
    fn split_whitespace(self) {}
}

struct DerefStr(&'static str);
impl std::ops::Deref for DerefStr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

struct DerefStrAndCustom(&'static str);
impl std::ops::Deref for DerefStrAndCustom {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl DerefStrAndCustom {
    fn trim(self) -> Self {
        self
    }
    fn split_whitespace(self) {}
}

struct DerefStrAndCustomSplit(&'static str);
impl std::ops::Deref for DerefStrAndCustomSplit {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl DerefStrAndCustomSplit {
    #[allow(dead_code)]
    fn split_whitespace(self) {}
}

struct DerefStrAndCustomTrim(&'static str);
impl std::ops::Deref for DerefStrAndCustomTrim {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl DerefStrAndCustomTrim {
    fn trim(self) -> Self {
        self
    }
}

fn main() {
    // &str
    let _ = " A B C ".trim().split_whitespace(); // should trigger lint
    let _ = " A B C ".trim_start().split_whitespace(); // should trigger lint
    let _ = " A B C ".trim_end().split_whitespace(); // should trigger lint

    // String
    let _ = (" A B C ").to_string().trim().split_whitespace(); // should trigger lint
    let _ = (" A B C ").to_string().trim_start().split_whitespace(); // should trigger lint
    let _ = (" A B C ").to_string().trim_end().split_whitespace(); // should trigger lint

    // Custom
    let _ = Custom.trim().split_whitespace(); // should not trigger lint

    // Deref<Target=str>
    let s = DerefStr(" A B C ");
    let _ = s.trim().split_whitespace(); // should trigger lint

    // Deref<Target=str> + custom impl
    let s = DerefStrAndCustom(" A B C ");
    let _ = s.trim().split_whitespace(); // should not trigger lint

    // Deref<Target=str> + only custom split_ws() impl
    let s = DerefStrAndCustomSplit(" A B C ");
    let _ = s.trim().split_whitespace(); // should trigger lint
    // Expl: trim() is called on str (deref) and returns &str.
    //       Thus split_ws() is called on str as well and the custom impl on S is unused

    // Deref<Target=str> + only custom trim() impl
    let s = DerefStrAndCustomTrim(" A B C ");
    let _ = s.trim().split_whitespace(); // should not trigger lint
}
