use pulldown_cmark::{html, Parser};

wit_bindgen_rust::export!("../renderer.wit");

struct Renderer;

impl renderer::Renderer for Renderer {
    fn render(input: String) -> String {
        let parser = Parser::new(&input);
        let mut output = String::new();
        html::push_html(&mut output, parser);
        output
    }
    fn name() -> String {
        "Markdown".to_string()
    }
}
