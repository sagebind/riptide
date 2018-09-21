use source::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct TokenInfo {
    pub token: Token,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Indicates the beginning of a sub-expression.
    LeftParen,
    /// Closes a sub-expression.
    RightParen,
    /// Indicates the beginning of a block.
    LeftBrace,
    /// Closes a block.
    RightBrace,
    /// Prefixes a named argument list for the following block.
    LeftBracket,
    /// Closes a named argument list.
    RightBracket,
    /// Semicolon separating statements.
    EndOfStatement,
    /// Separates function calls in a pipeline.
    Pipe,
    /// Separates the format specifier from the variable name in a format substitution.
    Colon,
    /// Delimits an interpolated string.
    DoubleQuote,
    /// Prefixes simple variable substitution.
    SubstitutionSigil,
    /// Indicates the beginning of a complex expression substitution.
    SubstitutionParen,
    /// Indicates the beginning of a string format substitution.
    SubstitutionBrace,
    /// A number literal.
    Number(f64),
    /// A string literal.
    StringLiteral(String),
    /// A string with possible substitutions.
    #[deprecated]
    DoubleQuotedString(String),
    /// Newline separator.
    EndOfLine,
    /// Indicates the end of file has been reached and no more tokens will be produced.
    EndOfFile,
}
