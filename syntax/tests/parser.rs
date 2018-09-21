extern crate glob;
#[macro_use]
extern crate log;
extern crate riptide_syntax;
extern crate riptide_syntax_extra;
extern crate stderrlog;
extern crate toml;
extern crate xmltree;

use riptide_syntax::parse;
use riptide_syntax::source::*;
use riptide_syntax_extra::xml::AsXml;
use std::fs;
use std::io::Cursor;

#[test]
pub fn run_all_tests() {
    stderrlog::new()
        .verbosity(3)
        .init()
        .unwrap();

    for path in glob::glob("tests/parser/**/*.toml").unwrap().filter_map(Result::ok) {
        let test = fs::read_to_string(&path).unwrap().parse::<toml::Value>().unwrap();
        let src = SourceFile::buffer(path.display().to_string(), test["source"].as_str().unwrap());

        if test.get("disabled").and_then(toml::Value::as_bool) == Some(true) {
            info!("skipping test: {}", src.name());
            continue;
        }

        let ast_xml = test["ast"].as_str().unwrap();
        let ast = xmltree::Element::parse(Cursor::new(ast_xml.as_bytes())).unwrap();

        info!("running test: {}", src.name());
        let actual = parse(src).unwrap().as_xml();

        if actual != ast {
            panic!(
                "AST are not equal!\n--ACUAL--\n{}\n--EXPECTED--\n{}",
                pretty_print_xml(&actual),
                pretty_print_xml(&ast),
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
