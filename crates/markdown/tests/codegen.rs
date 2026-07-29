use wit_bindgen_core::{Files, wit_parser::Resolve};

#[test]
fn doc_line_starting_with_closing_brace() {
    const WIT: &str = r#"
        package a:b;

        world w {
          export x: interface {
            record r {
              /// }
              f: u32,
            }
          }
        }
    "#;

    let mut resolve = Resolve::default();
    let package = resolve.push_str("test.wit", WIT).unwrap();
    let world = resolve.select_world(&[package], Some("w")).unwrap();
    let mut files = Files::default();
    let mut generator = wit_bindgen_markdown::Opts::default().build();

    generator.generate(&mut resolve, world, &mut files).unwrap();

    let markdown = String::from_utf8(files.remove("w.md").unwrap()).unwrap();
    assert!(markdown.contains("\n  <p>}\n"));
}
