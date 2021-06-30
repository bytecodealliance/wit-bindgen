use pulldown_cmark::{html, Parser};

witx_bindgen_rust::export!("markdown.witx");

struct Component;

impl markdown::Markdown for Component {
    fn render(&self, input: String) -> String {
        let parser = Parser::new(&input);
        let mut output = String::new();
        html::push_html(&mut output, parser);
        output
    }
}

fn markdown() -> &'static impl markdown::Markdown {
    static INSTANCE: Component = Component;
    &INSTANCE
}
