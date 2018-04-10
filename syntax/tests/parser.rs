extern crate riptide_syntax;

use riptide_syntax::ast::*;
use riptide_syntax::filemap::FileMap;
use riptide_syntax::parse;

#[test]
fn parse_string() {
    let file = FileMap::buffer(None, "
        'hello world'
    ");

    assert_eq!(parse(file).unwrap(), Block {
        named_params: None,
        statements: vec![
            Pipeline {
                items: vec![
                    Call {
                        function: Box::new(Expr::String("hello world".into())),
                        args: vec![],
                    }
                ]
            }
        ],
    });
}

#[test]
fn nested_function_calls() {
    let file = FileMap::buffer(None, "
        println hello ({read} THE) (uppercase World)
    ");

    assert_eq!(parse(file).unwrap(), Block {
        named_params: None,
        statements: vec![
            Pipeline {
                items: vec![
                    Call {
                        function: Box::new(Expr::String("println".into())),
                        args: vec![
                            Expr::String("hello".into()),
                            Expr::Pipeline(Pipeline {
                                items: vec![
                                    Call {
                                        function: Box::new(Expr::Block(Block {
                                            named_params: None,
                                            statements: vec![
                                                Pipeline {
                                                    items: vec![
                                                        Call {
                                                            function: Box::new(Expr::String("read".into())),
                                                            args: vec![],
                                                        },
                                                    ],
                                                },
                                            ],
                                        })),
                                        args: vec![
                                            Expr::String("THE".into()),
                                        ],
                                    },
                                ],
                            }),
                            Expr::Pipeline(Pipeline {
                                items: vec![
                                    Call {
                                        function: Box::new(Expr::String("uppercase".into())),
                                        args: vec![
                                            Expr::String("World".into()),
                                        ],
                                    },
                                ],
                            }),
                        ],
                    },
                ],
            },
        ],
    });
}
