use crate::{
    ast::*,
    error::ParseError,
    grammar::{self, Rule},
    source::{SourceFile, Span},
};
use pest::iterators::Pair;
use regex::bytes::Regex;

/// Attempt to parse a source file into an abstract syntax tree.
///
/// If the given file contains a valid Riptide program, a root AST node is
/// returned representing the program. If the program instead contains any
/// syntax errors, the errors are returned instead.
pub fn parse(source_file: impl Into<SourceFile>) -> Result<Block, ParseError> {
    let source_file = source_file.into();

    let mut ctx = ParsingContext {
        source_file: source_file.clone(),
    };

    let mut pair = match grammar::parse(source_file.source_text(), Rule::program) {
        Ok(pair) => pair,
        Err(e) => {
            let span = match e.location {
                pest::error::InputLocation::Pos(offset) => source_file.slice(offset, offset).unwrap(),
                pest::error::InputLocation::Span((start, end)) => source_file.slice(start, end).unwrap(),
            };

            return Err(ParseError::from_pest(span, e));
        }
    };

    from_pair(pair.next().unwrap(), &mut ctx)
}

/// An AST node that can be parsed.
///
/// Which is all of them...
trait ParsableNode: Sized {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError>;
}

/// Convenience function for `ParsableNode` that saves some typing.
fn from_pair<T: ParsableNode>(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<T, ParseError> {
    T::from_pair(pair, ctx)
}

/// Contextual information needed for parsing.
struct ParsingContext {
    /// Source file currently being parsed. This is provided so that the AST
    /// can fetch span information.
    source_file: SourceFile,
}

impl ParsingContext {
    fn span(&self, pair: &Pair<'_, Rule>) -> Span {
        let span = pair.as_span();

        self.source_file.slice(
            span.start_pos().pos(),
            span.end_pos().pos(),
        ).unwrap()
    }
}

impl ParsableNode for Block {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert!(pair.as_rule() == Rule::program || pair.as_rule() == Rule::block);

        let span = ctx.span(&pair);
        let mut pairs = pair.into_inner().collect::<Vec<_>>();

        if pairs.last().map(|pair| pair.as_rule() == Rule::EOI).unwrap_or(false) {
            pairs.pop();
        }

        let statements = pairs.pop().unwrap()
            .into_inner()
            .map(|p| from_pair(p, ctx))
            .collect::<Result<_, ParseError>>()?;

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
            span: Some(span),
            named_params,
            vararg_param,
            statements,
        })
    }
}

impl ParsableNode for Subroutine {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert!(pair.as_rule() == Rule::subroutine);

        let mut pairs = pair.into_inner();

        Ok(Subroutine {
            name: pairs.next().unwrap().as_str().to_owned(),
            block: from_pair(pairs.next().unwrap(), ctx)?,
        })
    }
}

impl ParsableNode for Statement {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        match pair.as_rule() {
            Rule::import_statement => Ok(Statement::Import(from_pair(pair, ctx)?)),
            Rule::return_statement => Ok(Statement::Return({
                let mut pairs = pair.into_inner();

                if let Some(pair) = pairs.next() {
                    Some(from_pair(pair, ctx)?)
                } else {
                    None
                }
            })),
            Rule::pipeline_statement => Ok(Statement::Pipeline(from_pair(pair.into_inner().next().unwrap(), ctx)?)),
            Rule::assignment_statement => {
                let mut pairs = pair.into_inner();

                Ok(Statement::Assignment(AssignmentStatement {
                    target: from_pair(pairs.next().unwrap(), ctx)?,
                    value: from_pair(pairs.next().unwrap(), ctx)?,
                }))
            },
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl ParsableNode for ImportStatement {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::import_statement);

        let mut pairs = pair.into_inner();

        Ok(ImportStatement {
            path: string_literal(pairs.next().unwrap()),
            clause: from_pair(pairs.next().unwrap(), ctx)?,
        })
    }
}

impl ParsableNode for ImportClause {
    fn from_pair(pair: Pair<'_, Rule>, _ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::import_clause);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::import_items => {
                let mut imports = Vec::new();

                for pair in pair.into_inner() {
                    imports.push(string_literal(pair));
                }

                Ok(ImportClause::Items(imports))
            }

            Rule::import_wildcard => Ok(ImportClause::Wildcard),

            _ => unreachable!()
        }
    }
}

impl ParsableNode for AssignmentTarget {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::assignment_target);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::member_access_expr => Ok(AssignmentTarget::MemberAccess(from_pair(pair, ctx)?)),
            Rule::variable_substitution => Ok(AssignmentTarget::Variable(string_literal(pair.into_inner().next().unwrap()))),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl ParsableNode for Pipeline {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::pipeline);

        Ok(Pipeline(pair.into_inner().map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?))
    }
}

impl ParsableNode for Call {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::call);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::named_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Named {
                    function: string_literal(pairs.next().unwrap()),
                    args: pairs.map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?,
                })
            }
            Rule::unnamed_call => {
                let mut pairs = pair.into_inner();

                Ok(Call::Unnamed {
                    function: Box::new(pairs.next().map(|p| from_pair(p, ctx)).unwrap()?),
                    args: pairs.map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?,
                })
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl ParsableNode for CallArg {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::call_arg);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::splat_arg => Ok(CallArg::Splat(from_pair(pair.into_inner().next().unwrap(), ctx)?)),
            Rule::expr => Ok(CallArg::Expr(from_pair(pair, ctx)?)),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl Expr {
    // TODO: Remove this
    fn from_pair_inner(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        Ok(match pair.as_rule() {
            Rule::block => Expr::Block(from_pair(pair, ctx)?),
            Rule::subroutine => Expr::Subroutine(from_pair(pair, ctx)?),
            Rule::pipeline => Expr::Pipeline(from_pair(pair, ctx)?),
            Rule::member_access_expr => from_pair(pair, ctx).map(Expr::MemberAccess)?,
            Rule::cvar => Expr::CvarReference(from_pair(pair, ctx)?),
            Rule::cvar_scope => Expr::CvarScope(from_pair(pair, ctx)?),
            Rule::regex_literal => Expr::Regex(from_pair(pair, ctx)?),
            Rule::substitution => Expr::Substitution(from_pair(pair, ctx)?),
            Rule::table_literal => Expr::Table(from_pair(pair, ctx)?),
            Rule::list_literal => Expr::List(from_pair(pair, ctx)?),
            Rule::interpolated_string => Expr::InterpolatedString(from_pair(pair, ctx)?),
            Rule::string_literal => Expr::String(string_literal(pair)),
            Rule::number_literal => Expr::Number(pair.as_str().parse().unwrap()),
            rule => panic!("unexpected rule: {:?}", rule),
        })
    }
}

impl ParsableNode for Expr {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert!(matches!(pair.as_rule(), Rule::expr));

        Self::from_pair_inner(pair.into_inner().next().unwrap(), ctx)
    }
}

impl ParsableNode for MemberAccess {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::member_access_expr);

        let mut pairs = pair.into_inner();
        let mut member_access = MemberAccess(
            Box::new(Expr::from_pair_inner(pairs.next().unwrap(), ctx)?),
            string_literal(pairs.next().unwrap()),
        );

        for member_name in pairs {
            member_access = MemberAccess(Box::new(Expr::MemberAccess(member_access)), string_literal(member_name));
        }

        Ok(member_access)
    }
}

impl ParsableNode for CvarReference {
    fn from_pair(pair: Pair<'_, Rule>, _ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        Ok(CvarReference(string_literal(pair)))
    }
}

impl ParsableNode for CvarScope {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        let mut pairs = pair.into_inner();

        Ok(CvarScope {
            name: from_pair(pairs.next().unwrap(), ctx)?,
            value: Box::new(from_pair(pairs.next().unwrap(), ctx)?),
            scope: from_pair(pairs.next().unwrap(), ctx)?,
        })
    }
}

impl ParsableNode for Substitution {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
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
                Ok(Substitution::Pipeline(from_pair(pair.into_inner().next().unwrap(), ctx)?))
            }
            Rule::variable_substitution => {
                Ok(Substitution::Variable(string_literal(pair.into_inner().next().unwrap())))
            }
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl ParsableNode for TableLiteral {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::table_literal);

        Ok(TableLiteral(pair.into_inner().map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?))
    }
}

impl ParsableNode for TableEntry {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::table_literal_entry);

        let mut pairs = pair.into_inner();

        Ok(TableEntry {
            key: from_pair(pairs.next().unwrap(), ctx)?,
            value: from_pair(pairs.next().unwrap(), ctx)?,
        })
    }
}

impl ParsableNode for ListLiteral {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::list_literal);

        Ok(ListLiteral(pair.into_inner().map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?))
    }
}

impl ParsableNode for InterpolatedString {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string);

        Ok(InterpolatedString(pair.into_inner().map(|p| from_pair(p, ctx)).collect::<Result<_, _>>()?))
    }
}

impl ParsableNode for InterpolatedStringPart {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        assert_eq!(pair.as_rule(), Rule::interpolated_string_part);

        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::substitution => from_pair(pair, ctx).map(InterpolatedStringPart::Substitution),
            Rule::interpolated_string_literal_part => Ok(InterpolatedStringPart::String(translate_escapes(pair.as_str()))),
            rule => panic!("unexpected rule: {:?}", rule),
        }
    }
}

impl ParsableNode for RegexLiteral {
    fn from_pair(pair: Pair<'_, Rule>, ctx: &mut ParsingContext) -> Result<Self, ParseError> {
        let regex_str = pair.as_str();
        let regex_str = &regex_str[1..regex_str.len()-1];

        match Regex::new(regex_str) {
            Ok(regex) => Ok(RegexLiteral(regex)),
            Err(e) => Err(ParseError::new(ctx.span(&pair), e.to_string()))
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
