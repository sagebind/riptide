extern crate glob;
extern crate riptide_syntax;
extern crate riptide_syntax_extra;
extern crate riptide_testing;
extern crate stderrlog;
extern crate xmltree;

use riptide_syntax::parse;
use riptide_syntax::source::*;
use riptide_syntax_extra::xml::AsXml;
use riptide_testing::TestFile;

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

#[test]
pub fn run_all_tests() {
    stderrlog::new()
        .verbosity(3)
        .init()
        .unwrap();

    for entry in glob::glob("tests/parser/**/*.test").unwrap() {
        let test = ParserTest::from(TestFile::load(entry.unwrap()).unwrap());
        test.run();
    }
}

fn pretty_print_xml(element: &xmltree::Element) -> String {
    let config = xmltree::EmitterConfig::new()
        .perform_indent(true);

    let mut buf = Vec::new();
    element.write_with_config(&mut buf, config).unwrap();
    String::from_utf8(buf).unwrap()
}
