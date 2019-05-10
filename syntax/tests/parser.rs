use log::*;
use riptide_syntax::ast;
use riptide_syntax::parse;
use riptide_syntax::source::*;
use serde::Serialize;
use std::fs;

#[derive(serde::Serialize, serde::Deserialize)]
struct ParserTest {
    #[serde(default)]
    skip: bool,
    source: String,
    ast: String,
}

impl ParserTest {
    fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Box<std::error::Error>> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}

#[test]
pub fn run_all_tests() -> Result<(), Box<std::error::Error>> {
    stderrlog::new().verbosity(3).init()?;

    for path in glob::glob("tests/parser/**/*.toml").unwrap().filter_map(Result::ok) {
        let test = ParserTest::load(&path)?;
        let src = SourceFile::named(path.display().to_string(), test.source.clone());

        if test.skip {
            info!("skipping test: {}", src.name());
            continue;
        }

        info!("running test: {}", src.name());

        let ast = parse(src.clone()).unwrap();
        let actual = serialize_ast(&ast);
        let expected = test.ast.trim();


        if actual != expected {
            eprintln!("{}", difference::Changeset::new(&expected, &actual, "\n"));
            panic!("actual AST does not match expected AST");
        }
    }

    Ok(())
}

fn serialize_ast(ast: &ast::Block) -> String {
    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
    ast.serialize(&mut serializer).unwrap();
    serializer.into_output_string()
}
