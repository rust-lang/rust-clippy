use crate::utils::File;
use core::fmt::{self, Display};
use memchr::memchr_iter;
use rustc_index::{IndexVec, newtype_index};
use std::path::{Path, PathBuf};

pub struct SourceData {
    pub contents: String,
    pub line_starts: Vec<u32>,
    pub path: PathBuf,
    pub module: String,
    pub krate: Crate,
}
impl SourceData {
    pub fn line_col(&self, pos: u32) -> (u32, u32) {
        #[expect(clippy::cast_possible_truncation)]
        let (line, offset) = match self.line_starts.binary_search(&pos) {
            Ok(i) => (i as u32 + 1, self.line_starts[i]),
            Err(i) => (i as u32, self.line_starts[i - 1]),
        };
        let mut col = 1;
        let mut remain = pos - offset;
        let mut chars = self.contents[offset as usize..].chars();
        #[expect(clippy::cast_possible_truncation)]
        while remain != 0
            && let Some(c) = chars.next()
        {
            col += 1;
            remain = remain.saturating_sub(c.len_utf8() as u32);
        }
        (line, col)
    }
}

pub struct SourceMap {
    pub crates: IndexVec<Crate, Box<str>>,
    pub files: IndexVec<SourceFile, SourceData>,
}
impl SourceMap {
    pub fn with_capacity(crates: usize, files: usize) -> Self {
        Self {
            crates: IndexVec::with_capacity(crates),
            files: IndexVec::with_capacity(files),
        }
    }

    pub fn add_new_crate(&mut self, name: &str) -> Crate {
        let res = self.crates.next_index();
        self.crates.push(name.into());
        res
    }

    pub fn add_crate(&mut self, name: &str) -> Crate {
        match self.crates.iter().position(|x| **x == *name) {
            Some(x) => Crate::from_usize(x),
            None => self.add_new_crate(name),
        }
    }

    pub fn load_new_file(&mut self, path: &Path, krate: Crate, module: String) -> SourceFile {
        let mut contents = String::new();
        File::open_read(path).read_append_to_string(&mut contents);

        let res = self.files.next_index();
        let mut line_starts = Vec::with_capacity(16);
        line_starts.push(0);
        #[expect(clippy::cast_possible_truncation)]
        line_starts.extend(memchr_iter(b'\n', contents.as_bytes()).map(|x| x as u32 + 1));
        self.files.push(SourceData {
            contents,
            line_starts,
            path: path.into(),
            module,
            krate,
        });
        res
    }

    pub fn load_file(&mut self, path: &Path, krate: Crate, module: &str) -> SourceFile {
        match self.files.iter().position(|x| x.krate == krate && x.module == module) {
            Some(x) => SourceFile::from_usize(x),
            None => self.load_new_file(path, krate, module.into()),
        }
    }
}

newtype_index! {
    pub struct SourceFile {}
}
newtype_index! {
    #[orderable]
    pub struct Crate {}
}

#[derive(Clone, Copy)]
pub struct Span {
    pub file: SourceFile,
    pub start: u32,
    pub end: u32,
}
impl Span {
    pub fn display(self, source_map: &SourceMap) -> impl use<'_> + Display {
        struct X<'a>(Span, &'a SourceMap);
        impl Display for X<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let file = &self.1.files[self.0.file];
                let (line, col) = file.line_col(self.0.start);
                write!(f, "{}:{line}:{col}", self.1.files[self.0.file].path.display())
            }
        }
        X(self, source_map)
    }
}
