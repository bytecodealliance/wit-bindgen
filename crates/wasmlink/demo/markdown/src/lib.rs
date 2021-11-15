use pulldown_cmark::{html, Parser};

wai_bindgen_rust::export!("markdown.wai");

struct Markdown;

impl markdown::Markdown for Markdown {
    fn render(input: String) -> String {
        let parser = Parser::new(&input);
        let mut output = String::new();
        html::push_html(&mut output, parser);
        output
    }
}
