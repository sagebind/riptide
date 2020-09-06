use pest::{
    error::Error,
    iterators::Pairs,
    Parser,
};

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
struct Grammar;

pub(crate) fn parse(input: &str, rule: Rule) -> Result<Pairs<'_, Rule>, Error<Rule>> {
    Grammar::parse(rule, input)
}
