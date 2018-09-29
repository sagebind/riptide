use ast::*;
use pest::iterators::Pair;

#[cfg(debug_assertions)]
const _GRAMMAR: &'static str = include_str!("grammar.pest");

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

        let statements = pairs.pop()
            .unwrap()
            .into_inner()
            .map(Pipeline::from_pair)
            .collect();

        let named_params = pairs.pop().map(|pair| {
            assert_eq!(pair.as_rule(), Rule::block_params);

            pair
                .into_inner()
                .map(|pair| pair.as_str().to_owned())
                .collect()
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

        Self {
            items: pair.into_inner().map(Call::from_pair).collect(),
        }
    }
}

impl FromPair for Call {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::call);

        let mut pairs = pair.into_inner();

        Self {
            function: Box::new(pairs.next().map(Expr::from_pair).unwrap()),
            args: pairs.map(Expr::from_pair).collect(),
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
            Rule::interpolation => Expr::InterpolatedString(InterpolatedString::from_pair(pair)),
            Rule::substitution => Expr::Substitution(Substitution::from_pair(pair)),
            Rule::string_literal => Expr::String(translate_escapes(pair.into_inner().next().unwrap().as_str())),
            Rule::number_literal => Expr::Number(pair.as_str().parse().unwrap()),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl FromPair for InterpolatedString {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::interpolation);

        InterpolatedString(pair.into_inner().map(InterpolatedStringPart::from_pair).collect())
    }
}

impl FromPair for InterpolatedStringPart {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::interpolation_part);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::substitution => InterpolatedStringPart::Substitution(Substitution::from_pair(pair)),
            Rule::interpolation_literal_part => InterpolatedStringPart::String(translate_escapes(pair.as_str())),
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
            },
            Rule::pipeline_substitution => Substitution::Pipeline(Pipeline::from_pair(pair.into_inner().next().unwrap())),
            Rule::variable_substitution => Substitution::Variable(VariablePath::from_pair(pair.into_inner().next().unwrap())),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl FromPair for VariablePath {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::variable_path);

        VariablePath(pair.into_inner().map(VariablePathPart::from_pair).collect())
    }
}

impl FromPair for VariablePathPart {
    fn from_pair(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::variable_path_part);

        VariablePathPart::Ident(pair.as_str().to_owned())
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
