use riptide_syntax::{
    ast,
    parse,
    source::*,
};
use serde::Serialize;
use std::{
    env,
    fs,
    error::Error,
    path::Path,
};

#[derive(serde::Serialize, serde::Deserialize)]
struct ParserTest {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    skip: bool,
    source: String,
    ast: String,
}

impl ParserTest {
    fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        Ok(fs::write(path, toml::to_string_pretty(self)?)?)
    }
}

#[test_generator::test_resources("syntax/tests/parser/**/*.toml")]
fn parser_test(path: &str) {
    let path = &path[7..];
    let mut test = ParserTest::load(path).unwrap();
    let src = SourceFile::named(path.to_string(), test.source.clone());

    if test.skip {
        println!("skipping test: {}", src.name());
        return;
    }

    let ast = parse(src.clone()).unwrap();
    let actual = serialize_ast(&ast);
    let expected = test.ast.trim();

    if env::var("PARSER_TEST_UPDATE").ok().as_deref() == Some("1") {
        test.ast = actual;
        test.ast.push('\n');
        println!("updating parser test: {}", path);
        test.save(path).unwrap();
    } else if actual != expected {
        eprintln!("{}", difference::Changeset::new(&expected, &actual, "\n"));
        panic!("actual AST does not match expected AST");
    }
}

fn serialize_ast(ast: &ast::Block) -> String {
    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
    ast.serialize(&mut serializer).unwrap();
    serializer.into_output_string()
}

fn is_false(b: &bool) -> bool {
    !*b
}
