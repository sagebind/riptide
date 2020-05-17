use crate::{
    ast::*,
    source::{Position, SourceFile, Span},
};
use pest::{
    error::Error,
    iterators::Pair,
    Parser as _,
};

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
struct Grammar;

pub(crate) struct Parser {
    source_file: SourceFile,
}

impl Parser {
    pub(crate) fn new(source_file: SourceFile) -> Self {
        Self {
            source_file,
        }
    }

    pub(crate) fn parse(&self) -> Result<Block, Error<Rule>> {
        self.parse_block(Grammar::parse(Rule::program, self.source_file.source())?.next().unwrap())
    }

    fn span(&self, pair: &Pair<'_, Rule>) -> Span {
        let span = pair.as_span();

        Span {
            file_name: Some(self.source_file.name().to_owned()),
            start: Position {
                line: span.start_pos().line_col().0 as u32,
                col: span.start_pos().line_col().1 as u32,
            },
            end: Position {
                line: span.end_pos().line_col().0 as u32,
                col: span.end_pos().line_col().1 as u32,
            },
        }
    }

    fn parse_block(&self, pair: Pair<'_, Rule>) -> Result<Block, Error<Rule>> {
        assert!(pair.as_rule() == Rule::program || pair.as_rule() == Rule::block);

        let span = self.span(&pair);
        let mut pairs = pair.into_inner().collect::<Vec<_>>();

        if pairs.last().map(|pair| pair.as_rule() == Rule::EOI).unwrap_or(false) {
            pairs.pop();
        }

        let statements = pairs.pop().unwrap()
            .into_inner()
            .map(|p| self.parse_statement(p))
            .collect::<Result<_, Error<Rule>>>()?;

        let mut named_params = None;
        let mut vararg_param = None;

        if let Some(block_params) = pairs.pop() {
            assert_eq!(block_params.as_rule(), Rule::block_params);

            for param in block_params.into_inner() {
                match param.as_rule() {
                    Rule::param_decl => {
                        named_params.get_or_insert_with(Vec::new).push(param.as_str().to_owned());
                    }
                    Rule::vararg_param_decl => {
                        vararg_param = Some(param.into_inner().next().unwrap().as_str().to_owned());
                    }
                    rule => panic!("unexpected rule: {:?}", rule),
                }
            }
        }

        Ok(Block {
            span,
            named_params,
            vararg_param,
            statements,
        })
    }

    fn parse_statement(&self, pair: Pair<'_, Rule>) -> Result<Statement, Error<Rule>> {
        match pair.as_rule() {
            Rule::pipeline_statement => Ok(Statement::Pipeline(self.parse_pipeline(pair.into_inner().next().unwrap())?)),
            Rule::assignment_statement => {
                let mut pairs = pair.into_inner();

                Ok(Statement::Assignment(AssignmentStatement {
                    target: self.parse_assignment_target(pairs.next().unwrap())?,
                    value: self.parse_expr(pairs.next().unwrap())?,
                }))
            },
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }

    fn parse_assignment_target(&self, pair: Pair<'_, Rule>) -> Result<AssignmentTarget, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::assignment_target);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::member_access_expr => Ok(AssignmentTarget::MemberAccess(self.parse_member_access(pair)?)),
            Rule::variable_substitution => Ok(AssignmentTarget::Variable(string_literal(pair.into_inner().next().unwrap()))),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }

    fn parse_pipeline(&self, pair: Pair<'_, Rule>) -> Result<Pipeline, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::pipeline);

        Ok(Pipeline(pair.into_inner().map(|p| self.parse_call(p)).collect::<Result<_, _>>()?))
    }

    fn parse_call(&self, pair: Pair<'_, Rule>) -> Result<Call, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::call);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::named_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Named {
                    function: string_literal(pairs.next().unwrap()),
                    args: pairs.map(|p| self.parse_expr(p)).collect::<Result<_, _>>()?,
                })
            }
            Rule::unnamed_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Unnamed {
                    function: Box::new(pairs.next().map(|p| self.parse_expr(p)).unwrap()?),
                    args: pairs.map(|p| self.parse_expr(p)).collect::<Result<_, _>>()?,
                })
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }

    fn parse_expr(&self, pair: Pair<'_, Rule>) -> Result<Expr, Error<Rule>> {
        assert!(matches!(pair.as_rule(), Rule::expr));

        self.parse_expr_inner(pair.into_inner().next().unwrap())
    }

    fn parse_expr_inner(&self, pair: Pair<'_, Rule>) -> Result<Expr, Error<Rule>> {
        Ok(match pair.as_rule() {
            Rule::block => Expr::Block(self.parse_block(pair)?),
            Rule::pipeline => Expr::Pipeline(self.parse_pipeline(pair)?),
            Rule::member_access_expr => self.parse_member_access(pair).map(Expr::MemberAccess)?,
            Rule::cvar => Expr::CvarReference(self.parse_cvar_reference(pair)?),
            Rule::cvar_scope => Expr::CvarScope(self.parse_cvar_scope(pair)?),
            Rule::substitution => Expr::Substitution(self.parse_substitution(pair)?),
            Rule::table_literal => Expr::Table(self.parse_table_literal(pair)?),
            Rule::list_literal => Expr::List(self.parse_list_literal(pair)?),
            Rule::interpolated_string => Expr::InterpolatedString(self.parse_interpolated_string(pair)?),
            Rule::string_literal => Expr::String(string_literal(pair)),
            Rule::number_literal => Expr::Number(pair.as_str().parse().unwrap()),
            rule => panic!("unexpected rule: {:?}", rule),
        })
    }

    fn parse_member_access(&self, pair: Pair<'_, Rule>) -> Result<MemberAccess, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::member_access_expr);

        let mut pairs = pair.into_inner();
        let mut member_access = MemberAccess(
            Box::new(self.parse_expr_inner(pairs.next().unwrap())?),
            string_literal(pairs.next().unwrap()),
        );

        for member_name in pairs {
            member_access = MemberAccess(Box::new(Expr::MemberAccess(member_access)), string_literal(member_name));
        }

        Ok(member_access)
    }

    fn parse_cvar_reference(&self, pair: Pair<'_, Rule>) -> Result<CvarReference, Error<Rule>> {
        Ok(CvarReference(string_literal(pair)))
    }

    fn parse_cvar_scope(&self, pair: Pair<'_, Rule>) -> Result<CvarScope, Error<Rule>> {
        let mut pairs = pair.into_inner();

        Ok(CvarScope {
            name: self.parse_cvar_reference(pairs.next().unwrap())?,
            value: Box::new(self.parse_expr(pairs.next().unwrap())?),
            scope: self.parse_block(pairs.next().unwrap())?,
        })
    }

    fn parse_substitution(&self, pair: Pair<'_, Rule>) -> Result<Substitution, Error<Rule>> {
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
                Ok(Substitution::Pipeline(self.parse_pipeline(pair.into_inner().next().unwrap())?))
            }
            Rule::variable_substitution => {
                Ok(Substitution::Variable(string_literal(pair.into_inner().next().unwrap())))
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }

    fn parse_table_literal(&self, pair: Pair<'_, Rule>) -> Result<TableLiteral, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::table_literal);

        Ok(TableLiteral(pair.into_inner().map(|p| self.parse_table_entry(p)).collect::<Result<_, _>>()?))
    }

    fn parse_table_entry(&self, pair: Pair<'_, Rule>) -> Result<TableEntry, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::table_literal_entry);

        let mut pairs = pair.into_inner();

        Ok(TableEntry {
            key: self.parse_expr(pairs.next().unwrap())?,
            value: self.parse_expr(pairs.next().unwrap())?,
        })
    }

    fn parse_list_literal(&self, pair: Pair<'_, Rule>) -> Result<ListLiteral, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::list_literal);

        Ok(ListLiteral(pair.into_inner().map(|p| self.parse_expr(p)).collect::<Result<_, _>>()?))
    }

    fn parse_interpolated_string(&self, pair: Pair<'_, Rule>) -> Result<InterpolatedString, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string);

        Ok(InterpolatedString(pair.into_inner().map(|p| self.parse_interpolated_string_part(p)).collect::<Result<_, _>>()?))
    }

    fn parse_interpolated_string_part(&self, pair: Pair<'_, Rule>) -> Result<InterpolatedStringPart, Error<Rule>> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string_part);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::substitution => self.parse_substitution(pair).map(InterpolatedStringPart::Substitution),
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
