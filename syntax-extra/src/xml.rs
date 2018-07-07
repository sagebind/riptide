use riptide_syntax::ast::*;
use xmltree::Element;

pub trait AsXml {
    fn as_xml(&self) -> Element;

    fn as_xml_string(&self) -> String {
        let mut buf = Vec::new();
        self.as_xml().write(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

impl AsXml for Block {
    fn as_xml(&self) -> Element {
        let mut root_element = Element::new("block");

        let mut named_params = Element::new("named-params");
        if let Some(ref params) = self.named_params {
            for named_param in params.iter() {
                named_params.children.push(Element::new("param"));
            }
        }
        root_element.children.push(named_params);

        let mut statements = Element::new("statements");
        for statement in self.statements.iter() {
            statements.children.push(statement.as_xml());
        }
        root_element.children.push(statements);

        root_element
    }
}

impl AsXml for Pipeline {
    fn as_xml(&self) -> Element {
        let mut root_element = Element::new("pipeline");

        for call in self.items.iter() {
            root_element.children.push(call.as_xml());
        }

        root_element
    }
}

impl AsXml for Call {
    fn as_xml(&self) -> Element {
        let mut root_element = Element::new("call");

        let mut function_element = Element::new("function");
        function_element.children.push(self.function.as_xml());
        root_element.children.push(function_element);

        let mut args_element = Element::new("args");
        for arg in self.args.iter() {
            args_element.children.push(arg.as_xml());
        }
        root_element.children.push(args_element);

        root_element
    }
}

impl AsXml for Expr {
    fn as_xml(&self) -> Element {
        match self {
            Expr::Block(block) => block.as_xml(),
            Expr::Pipeline(pipeline) => pipeline.as_xml(),
            Expr::String(string) => {
                let mut element = Element::new("string");
                element.attributes.insert(String::from("value"), string.clone());
                element
            },
            Expr::Number(number) => {
                let mut element = Element::new("number");
                element.attributes.insert(String::from("value"), number.to_string());
                element
            },
            _ => unimplemented!(),
        }
    }
}
