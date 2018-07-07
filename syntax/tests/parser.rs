extern crate riptide_syntax;
extern crate riptide_syntax_extra;
extern crate riptide_testing;
extern crate xmltree;

use riptide_syntax::parse;
use riptide_syntax::source::*;
use riptide_syntax_extra::xml::AsXml;
use riptide_testing::TestFile;

use std::fs;

#[test]
pub fn run_all_tests() {
    for entry in fs::read_dir("tests/parser").unwrap() {
        let entry = entry.unwrap();

        if entry.file_type().unwrap().is_file() {
            let test = ParserTest::from(TestFile::load(entry.path()).unwrap());
            test.run();
        }
    }
}

struct ParserTest {
    src: SourceFile,
    ast: xmltree::Element,
}

impl From<TestFile> for ParserTest {
    fn from(file: TestFile) -> Self {
        ParserTest {
            src: SourceFile::buffer(file.name().to_string(), file.get_section("SRC").unwrap()),
            ast: xmltree::Element::parse(file.get_section_reader("AST").unwrap()).unwrap(),
        }
    }
}

impl ParserTest {
    fn run(self) {
        let actual = parse(self.src).unwrap().as_xml();

        if actual != self.ast {
            panic!(
                "AST are not equal!\n--ACUAL--\n{}\n--EXPECTED--\n{}",
                pretty_print_xml(&actual),
                pretty_print_xml(&self.ast),
            );
        }
    }
}

fn pretty_print_xml(element: &xmltree::Element) -> String {
    let config = xmltree::EmitterConfig::new()
        .perform_indent(true);

    let mut buf = Vec::new();
    element.write_with_config(&mut buf, config).unwrap();
    String::from_utf8(buf).unwrap()
}
