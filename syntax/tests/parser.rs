extern crate difference;
extern crate glob;
#[macro_use]
extern crate log;
extern crate riptide_syntax;
extern crate stderrlog;
extern crate toml;

use riptide_syntax::parse;
use riptide_syntax::source::*;
use std::fs;

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

        info!("running test: {}", src.name());

        let expected = test["ast"].as_str().unwrap().trim();
        let actual = format!("{:#?}", parse(src).unwrap());

        if actual != expected {
            eprintln!("{}", difference::Changeset::new(&expected, &actual, "\n"));
            panic!("actual AST does not match expected AST");
        }
    }
}
