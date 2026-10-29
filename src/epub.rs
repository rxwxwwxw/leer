use anyhow::Result;
use chrono::Utc;
use maud::{html, PreEscaped};
use pulldown_cmark::{html as markdown_html, CowStr, Event, Options, Parser};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
use crate::models::story::{display_story_title, StoryDetail};

// XML 1.0 disallows most control characters, even when escaped.
fn xml_text(text: &str) -> String {
    text.chars().filter(|&c| matches!(c, '\u{9}' | '\u{a}' | '\u{d}') || (c >= '\u{20}' && c != '\u{fffe}' && c != '\u{ffff}')).collect()
}

/// Produce a portable, bounded filename while retaining Unicode for the browser.
pub fn download_filename(author: &str, title: &str) -> String {
    fn component(value: &str, fallback: &str) -> String {
        let mut cleaned = String::new();
        let mut pending_space = false;
        for ch in value.chars() {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_whitespace() {
                pending_space = !cleaned.is_empty();
                continue;
            }
            let extra = ch.len_utf8() + usize::from(pending_space);
            if cleaned.len() + extra > 100 { break; }
            if pending_space { cleaned.push(' '); pending_space = false; }
            cleaned.push(ch);
        }
        let cleaned = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
        if cleaned.is_empty() { fallback.to_string() } else { cleaned.to_string() }
    }
    format!("{} - {}.epub", component(author, "Unknown author"), component(title, "Untitled"))
}

pub fn create_epub(story: &StoryDetail) -> Result<Vec<u8>> {
    let title = xml_text(display_story_title(&story.title));
    let author = xml_text(&story.author);
    let text = xml_text(&story.content);
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_SMART_PUNCTUATION;
    // Raw HTML in source text may not be well-formed XML or safe for an ebook reader.
    let events = Parser::new_ext(&text, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(CowStr::from(raw.into_string())),
        other => other,
    });
    let mut body = String::new();
    markdown_html::push_html(&mut body, events);
    let chapter = html! {
        (PreEscaped("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"))
        html xmlns="http://www.w3.org/1999/xhtml" lang="en" {
            head { title { (&title) } }
            body { h1 { (&title) } p { (&author) } (PreEscaped(body)) }
        }
    }.into_string();
    let nav = html! {
        (PreEscaped("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"))
        html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="en" {
            head { title { "Contents" } }
            body { nav epub:type="toc" id="toc" { h1 { "Contents" } ol { li { a href="story.xhtml" { (&title) } } } } }
        }
    }.into_string();
    let modified = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let identifier = format!("urn:story:{}", story.id);
    let package = html! {
        (PreEscaped("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"))
        package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id" {
            metadata xmlns:dc="http://purl.org/dc/elements/1.1/" {
                dc:identifier id="book-id" { (identifier) }
                dc:title { (&title) }
                dc:creator { (&author) }
                dc:language { "en" }
                meta property="dcterms:modified" { (modified) }
            }
            manifest {
                item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav" {}
                item id="story" href="story.xhtml" media-type="application/xhtml+xml" {}
            }
            spine { itemref idref="story" {} }
        }
    }.into_string();
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("mimetype", stored)?;
    zip.write_all(b"application/epub+zip")?;
    zip.start_file("META-INF/container.xml", compressed)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#)?;
    for (name, content) in [("OEBPS/content.opf", package), ("OEBPS/nav.xhtml", nav), ("OEBPS/story.xhtml", chapter)] {
        zip.start_file(name, compressed)?;
        zip.write_all(content.as_bytes())?;
    }
    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn filenames_are_safe_and_keep_unicode() {
        assert_eq!(download_filename("Ursula Le Guin", "A Wizard of Earthsea"), "Ursula Le Guin - A Wizard of Earthsea.epub");
        assert_eq!(download_filename("José/\r\n", "../<Night>\\Day"), "José - Night Day.epub");
        assert_eq!(download_filename("", "..."), "Unknown author - Untitled.epub");
        let name = download_filename(&"é".repeat(100), &"x".repeat(200));
        assert!(name.len() <= 209);
        assert!(name.ends_with(".epub"));
    }
    #[test]
    fn archive_contains_uncompressed_mimetype_and_escaped_story() -> Result<()> {
        let story = StoryDetail { id: 42, title: "A & B".into(), author: "<author>".into(), datetime: "Unknown".into(), upvotes: 0, downvotes: 0, readers: 0, tags: None, keywords: None, introduction: None, content: "Hello <script>bad</script> & **welcome**".into() };
        let data = create_epub(&story)?;
        let mut archive = zip::ZipArchive::new(Cursor::new(data))?;
        let mut mime = archive.by_index(0)?;
        assert_eq!(mime.name(), "mimetype");
        assert_eq!(mime.compression(), CompressionMethod::Stored);
        let mut value = String::new();
        mime.read_to_string(&mut value)?;
        assert_eq!(value, "application/epub+zip");
        drop(mime);
        let mut body = String::new();
        archive.by_name("OEBPS/story.xhtml")?.read_to_string(&mut body)?;
        assert!(body.contains("A &amp; B"));
        assert!(body.contains("&lt;script&gt;"));
        assert!(body.contains("<strong>welcome</strong>"));
        Ok(())
    }
}
