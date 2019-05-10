use crate::ast::*;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct Grammar;

pub trait FromPair {
    fn from_pair<'a>(pair: Pair<'a, Rule>) -> Self;
}

impl FromPair for Block {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert!(pair.as_rule() == Rule::program || pair.as_rule() == Rule::block);

        let mut pairs = pair.into_inner().collect::<Vec<_>>();

        if pairs.last().map(|pair| pair.as_rule() == Rule::EOI).unwrap_or(false) {
            pairs.pop();
        }

        let statements = pairs.pop().unwrap().into_inner().map(Pipeline::from_pair).collect();

        let named_params = pairs.pop().map(|pair| {
            assert_eq!(pair.as_rule(), Rule::block_params);

            pair.into_inner().map(|pair| pair.as_str().to_owned()).collect()
        });

        Self {
            named_params,
            statements,
        }
    }
}

impl FromPair for Pipeline {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::pipeline);

        Pipeline(pair.into_inner().map(Call::from_pair).collect())
    }
}

impl FromPair for Call {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::call);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::named_call => {
                let mut pairs = pair.into_inner();

                Call::Named {
                    function: pairs.next().map(VariablePath::from_pair).unwrap(),
                    args: pairs.map(Expr::from_pair).collect(),
                }
            }
            Rule::unnamed_call => {
                let mut pairs = pair.into_inner();

                Call::Unnamed {
                    function: Box::new(pairs.next().map(Expr::from_pair).unwrap()),
                    args: pairs.map(Expr::from_pair).collect(),
                }
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl FromPair for Expr {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::expr);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::block => Expr::Block(Block::from_pair(pair)),
            Rule::pipeline => Expr::Pipeline(Pipeline::from_pair(pair)),
            Rule::substitution => Expr::Substitution(Substitution::from_pair(pair)),
            Rule::table_literal => Expr::Table(TableLiteral::from_pair(pair)),
            Rule::list_literal => Expr::List(ListLiteral::from_pair(pair)),
            Rule::interpolated_string => Expr::InterpolatedString(InterpolatedString::from_pair(pair)),
            Rule::string_literal => Expr::String(translate_escapes(pair.into_inner().next().unwrap().as_str())),
            Rule::number_literal => Expr::Number(pair.as_str().parse().unwrap()),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl FromPair for Substitution {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::substitution);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::format_substitution => {
                let mut pairs = pair.into_inner();
                let variable = pairs.next().map(VariablePath::from_pair).unwrap();
                let flags = pairs.next().map(|pair| pair.as_str().to_owned());

                Substitution::Format(variable, flags)
            }
            Rule::pipeline_substitution => {
                Substitution::Pipeline(Pipeline::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::variable_substitution => {
                Substitution::Variable(VariablePath::from_pair(pair.into_inner().next().unwrap()))
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl FromPair for VariablePath {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::variable_path);

        VariablePath(
            pair.into_inner().map(|pair| pair.into_inner().next().unwrap().as_str()).map(translate_escapes).collect(),
        )
    }
}

impl FromPair for TableLiteral {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::table_literal);

        TableLiteral(pair.into_inner().map(TableEntry::from_pair).collect())
    }
}

impl FromPair for TableEntry {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::table_literal_entry);

        let mut pairs = pair.into_inner();

        Self {
            key: pairs.next().map(Expr::from_pair).unwrap(),
            value: pairs.next().map(Expr::from_pair).unwrap(),
        }
    }
}

impl FromPair for ListLiteral {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::list_literal);

        ListLiteral(pair.into_inner().map(Expr::from_pair).collect())
    }
}

impl FromPair for InterpolatedString {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::interpolated_string);

        InterpolatedString(pair.into_inner().map(InterpolatedStringPart::from_pair).collect())
    }
}

impl FromPair for InterpolatedStringPart {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::interpolated_string_part);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::substitution => InterpolatedStringPart::Substitution(Substitution::from_pair(pair)),
            Rule::interpolated_string_literal_part => InterpolatedStringPart::String(translate_escapes(pair.as_str())),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

fn translate_escapes(source: &str) -> String {
    let mut string = String::with_capacity(source.len());
    let mut chars = source.chars();

    while let Some(c) = chars.next() {
        match c {
            '\\' => string.push(match chars.next().unwrap() {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                c => c, // interpret all other chars as their literal
            }),
            c => string.push(c),
        }
    }

    string
}
