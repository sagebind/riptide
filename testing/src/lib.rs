#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate xmltree;

use regex::bytes::*;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::Path;

pub struct TestFile {
    name: String,
    data: HashMap<String, Vec<u8>>,
}

impl TestFile {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        lazy_static! {
            static ref REGEX: Regex = Regex::new(r#"--(\w+)--\n"#).unwrap();
        }

        let mut data = HashMap::new();

        let path = path.as_ref();
        let name = path.display().to_string();

        let file = fs::read(path)?;
        let captures = REGEX.captures_iter(&file).collect::<Vec<_>>();

        let mut end = file.len();
        for capture in captures.iter().rev() {
            let capture_start = capture.get(0).unwrap().start();
            let capture_end = capture.get(0).unwrap().end();

            let name = String::from_utf8(capture[1].to_vec()).unwrap();
            let contents = file[capture_end..end].to_vec();

            data.insert(name, contents);
            end = capture_start;
        }

        Ok(TestFile {
            name,
            data,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_section(&self, name: &str) -> Option<&[u8]> {
        self.data.get(name).map(Vec::as_slice)
    }

    pub fn get_section_reader(&self, name: &str) -> Option<io::Cursor<&[u8]>> {
        self.get_section(name).map(io::Cursor::new)
    }
}
