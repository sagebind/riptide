use minidom::Element;
use quick_xml;
use riptide_syntax::ast::*;

pub trait AsXml {
    fn as_xml(&self) -> Element;

    fn as_xml_string(&self) -> String {
        let mut buf = Vec::new();
        self.as_xml().write_to(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn as_pretty_xml_string(&self) -> String {
        let mut buf = Vec::new();
        {
            let mut writer = quick_xml::Writer::new_with_indent(&mut buf, b' ', 4);
            self.as_xml().to_writer(&mut writer).unwrap();
        }
        String::from_utf8(buf).unwrap()
    }
}

impl AsXml for Element {
    fn as_xml(&self) -> Element {
        self.clone()
    }
}

impl AsXml for Block {
    fn as_xml(&self) -> Element {
        Element::builder("block")
            .append(Element::builder("named-params")
                .append(self.named_params
                    .iter()
                    .flat_map(|i| i)
                    .map(|name| Element::builder("param")
                        .attr("name", name)
                        .build())
                    .collect::<Vec<Element>>())
                .build())
            .append(Element::builder("statements")
                .append(self.statements.iter().map(AsXml::as_xml).collect::<Vec<Element>>())
                .build())
            .build()
    }
}

impl AsXml for Pipeline {
    fn as_xml(&self) -> Element {
        Element::builder("pipeline")
            .append(self.items.iter().map(AsXml::as_xml).collect::<Vec<Element>>())
            .build()
    }
}

impl AsXml for Call {
    fn as_xml(&self) -> Element {
        Element::builder("call")
            .append(Element::builder("function")
                .append(self.function.as_xml())
                .build())
            .append(Element::builder("args")
                .append(self.args.iter().map(AsXml::as_xml).collect::<Vec<Element>>())
                .build())
            .build()
    }
}

impl AsXml for Expr {
    fn as_xml(&self) -> Element {
        match self {
            Expr::Block(block) => block.as_xml(),
            Expr::Pipeline(pipeline) => pipeline.as_xml(),
            Expr::String(string) => Element::builder("string")
                .attr("value", string)
                .build(),
            Expr::Number(number) => Element::builder("number")
                .attr("value", number.to_string())
                .build(),
            Expr::Substitution(substitution) => substitution.as_xml(),
            _ => unimplemented!(),
        }
    }
}

impl AsXml for Substitution {
    fn as_xml(&self) -> Element {
        match self {
            Substitution::Variable(path) => Element::builder("substitution")
                .attr("variable", path.to_string())
                .build(),
            Substitution::Pipeline(pipeline) => Element::builder("substitution")
                .append(pipeline.as_xml())
                .build(),
            _ => unimplemented!(),
        }
    }
}
