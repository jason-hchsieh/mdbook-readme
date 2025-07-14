use std::io;
use std::io::Write;
use std::error::Error;
use mdbook::renderer::RenderContext;
use mdbook::BookItem;

fn main() {
    let mut stdin = io::stdin();
    let ctx = RenderContext::from_json(&mut stdin).unwrap();

    let built_ver = semver::Version::parse(mdbook::MDBOOK_VERSION)
        .expect("Failed to parse mdbook version");
    let book_ver = semver::Version::parse(&ctx.version)
        .expect("Failed to parse book version");
    if built_ver != book_ver {
        panic!("backend built on v{}, but mdbook on v{}",
            built_ver, book_ver);
    }

    let backend = OnePageBackend;
    backend.process(ctx);
}

#[derive(Debug)]
#[derive(PartialEq, Eq)]
#[derive(Copy, Clone)]
enum Chapter {
    Prefix,
    Numbered,
    Suffix,
    Draft,
}

#[derive(Debug)]
struct OnePageBackend;

impl OnePageBackend {
    fn process(&self, ctx: RenderContext) {
        // Specify root of the link in README.md
        let root = if let Some(tbl) = ctx.config.get_renderer("readme") {
            if let Some(toml::value::Value::String(s)) = tbl.get("root") {
                std::path::PathBuf::from(s)
            } else {
                std::path::PathBuf::from(".")
            }
        } else {
            std::path::PathBuf::from(".")
        };

        let output_file = ctx.destination.clone().join("README.md");
        let mut output_file = std::fs::File::create(&output_file)
            .expect("Failed to create README.md");

        let mut prev = Chapter::Prefix;

        for item in ctx.book.iter() {
            prev = (|item: &BookItem, prev: Chapter| -> Result<Chapter, Box<dyn Error>> {
                let mut cur = prev;
                match item {
                    BookItem::Chapter(ch) => {
                        // Chapter without number may be prefix or suffix
                        // Suffix chapter is followed by numbered chapters
                        if ch.number.is_none() {
                            if prev == Chapter::Numbered {
                                cur = Chapter::Suffix;
                            } else {
                                cur = Chapter::Prefix;
                            }
                        } else {
                            cur = Chapter::Numbered;
                        }

                        // Draft prev has no path
                        if ch.path.is_none() {
                            cur = Chapter::Draft;
                        }
                         
                        // Insert newline after numbered chapters
                        if prev == Chapter::Numbered && cur == Chapter::Suffix {
                            writeln!(output_file, "")?;
                        }

                        match cur {
                            Chapter::Prefix => {
                                writeln!(output_file, "{}", ch.content)?;
                                writeln!(output_file, "")?;
                            }
                            Chapter::Numbered => {
                                let indent = ch.number.as_ref().expect("Failed to get section number").len();
                                let indent = (indent - 1) * 2;
                                let indent = std::iter::repeat(' ')
                                    .take(indent)
                                    .collect::<String>();
                                write!(output_file, "{}", indent)?; 
                                write!(output_file, "-")?;
                                let path = ch.source_path.as_ref().expect("Failed to get source path");
                                let path = root.clone().join(path);
                                writeln!(output_file, " [{}]({})",
                                  ch.name, path.display())?;
                            }
                            Chapter::Suffix => {
                                writeln!(output_file, "{}", ch.content)?;
                                writeln!(output_file, "")?;
                            }
                            Chapter::Draft => {}
                        }
                    }
                    BookItem::Separator => {
                        // Insert newline after numbered chapters
                        if prev == Chapter::Numbered {
                            writeln!(output_file, "")?;
                        }
                        writeln!(output_file, "---")?;
                        writeln!(output_file, "")?;
                    }
                    BookItem::PartTitle(title) => {
                        if prev == Chapter::Numbered {
                            writeln!(output_file, "")?;
                        }
                        writeln!(output_file, "# {}", title)?;
                        writeln!(output_file, "")?;
                    }
                };
                Ok(cur)
            })(item, prev).expect("Failed to write");
        }
    }
}
