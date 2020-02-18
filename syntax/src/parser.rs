use crate::ast::*;
use pest::error::Error;
use pest::iterators::Pair;
use std::convert::TryFrom;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct Grammar;

impl TryFrom<Pair<'_, Rule>> for Block {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert!(pair.as_rule() == Rule::program || pair.as_rule() == Rule::block);

        let mut pairs = pair.into_inner().collect::<Vec<_>>();

        if pairs.last().map(|pair| pair.as_rule() == Rule::EOI).unwrap_or(false) {
            pairs.pop();
        }

        let statements = pairs.pop().unwrap()
            .into_inner()
            .map(Pipeline::try_from)
            .collect::<Result<_, Error<Rule>>>()?;

        let named_params = pairs.pop().map(|pair| {
            assert_eq!(pair.as_rule(), Rule::block_params);

            pair.into_inner().map(|pair| pair.as_str().to_owned()).collect()
        });

        Ok(Self {
            named_params,
            statements,
        })
    }
}

impl TryFrom<Pair<'_, Rule>> for Pipeline {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::pipeline);

        Ok(Pipeline(pair.into_inner().map(Call::try_from).collect::<Result<_, _>>()?))
    }
}

impl TryFrom<Pair<'_, Rule>> for Call {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::call);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::named_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Named {
                    function: string_literal(pairs.next().unwrap()),
                    args: pairs.map(Expr::try_from).collect::<Result<_, _>>()?,
                })
            }
            Rule::unnamed_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Unnamed {
                    function: Box::new(pairs.next().map(Expr::try_from).unwrap()?),
                    args: pairs.map(Expr::try_from).collect::<Result<_, _>>()?,
                })
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl TryFrom<Pair<'_, Rule>> for Expr {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        fn from_inner(pair: Pair<'_, Rule>) -> Result<Expr, Error<Rule>> {
            Ok(match pair.as_rule() {
                Rule::block => Expr::Block(Block::try_from(pair)?),
                Rule::pipeline => Expr::Pipeline(Pipeline::try_from(pair)?),
                Rule::member_access_expr => {
                    let mut pairs = pair.into_inner();
                    let mut expr = from_inner(pairs.next().unwrap())?;

                    for member_name in pairs {
                        expr = Expr::MemberAccess(MemberAccess(Box::new(expr), string_literal(member_name)));
                    }

                    expr
                },
                Rule::cvar => Expr::CvarReference(CvarReference::try_from(pair)?),
                Rule::cvar_scope => Expr::CvarScope(CvarScope::try_from(pair)?),
                Rule::substitution => Expr::Substitution(Substitution::try_from(pair)?),
                Rule::table_literal => Expr::Table(TableLiteral::try_from(pair)?),
                Rule::list_literal => Expr::List(ListLiteral::try_from(pair)?),
                Rule::interpolated_string => Expr::InterpolatedString(InterpolatedString::try_from(pair)?),
                Rule::string_literal => Expr::String(string_literal(pair)),
                Rule::number_literal => Expr::Number(pair.as_str().parse().unwrap()),
                rule => panic!("unexpected rule: {:?}", rule),
            })
        }

        assert_eq!(pair.as_rule(), Rule::expr);
        from_inner(pair.into_inner().next().unwrap())
    }
}

impl TryFrom<Pair<'_, Rule>> for CvarReference {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        Ok(CvarReference(string_literal(pair)))
    }
}

impl TryFrom<Pair<'_, Rule>> for CvarScope {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        let mut pairs = pair.into_inner();

        Ok(CvarScope {
            name: CvarReference::try_from(pairs.next().unwrap())?,
            value: Box::new(Expr::try_from(pairs.next().unwrap())?),
            scope: Block::try_from(pairs.next().unwrap())?,
        })
    }
}

impl TryFrom<Pair<'_, Rule>> for Substitution {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::substitution);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::format_substitution => {
                let mut pairs = pair.into_inner();
                let variable = pairs.next().map(string_literal).unwrap();
                let flags = pairs.next().map(|pair| pair.as_str().to_owned());

                Ok(Substitution::Format(variable, flags))
            }
            Rule::pipeline_substitution => {
                Ok(Substitution::Pipeline(Pipeline::try_from(pair.into_inner().next().unwrap())?))
            }
            Rule::variable_substitution => {
                Ok(Substitution::Variable(string_literal(pair.into_inner().next().unwrap())))
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl TryFrom<Pair<'_, Rule>> for TableLiteral {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::table_literal);

        Ok(TableLiteral(pair.into_inner().map(TableEntry::try_from).collect::<Result<_, _>>()?))
    }
}

impl TryFrom<Pair<'_, Rule>> for TableEntry {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::table_literal_entry);

        let mut pairs = pair.into_inner();

        Ok(Self {
            key: pairs.next().map(Expr::try_from).unwrap()?,
            value: pairs.next().map(Expr::try_from).unwrap()?,
        })
    }
}

impl TryFrom<Pair<'_, Rule>> for ListLiteral {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::list_literal);

        Ok(ListLiteral(pair.into_inner().map(Expr::try_from).collect::<Result<_, _>>()?))
    }
}

impl TryFrom<Pair<'_, Rule>> for InterpolatedString {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string);

        Ok(InterpolatedString(pair.into_inner().map(InterpolatedStringPart::try_from).collect::<Result<_, _>>()?))
    }
}

impl TryFrom<Pair<'_, Rule>> for InterpolatedStringPart {
    type Error = Error<Rule>;

    fn try_from(pair: Pair<'_, Rule>) -> Result<Self, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string_part);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::substitution => Substitution::try_from(pair).map(InterpolatedStringPart::Substitution),
            Rule::interpolated_string_literal_part => Ok(InterpolatedStringPart::String(translate_escapes(pair.as_str()))),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

fn string_literal(pair: Pair<'_, Rule>) -> String {
    translate_escapes(pair.into_inner().next().unwrap().as_str())
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
