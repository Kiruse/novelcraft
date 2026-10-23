//! Minimal markdown-like parser tuned for streaming agent output.
//!
//! Deliberately NOT CommonMark — only a small subset is supported:
//!
//! - paragraphs (blank-line separated; soft line breaks join with a space)
//! - `#`..`######` headings
//! - `>` blockquotes (contiguous groups; inner lines re-parsed as blocks)
//! - `**bold**` and `*italic*`
//! - double-quoted multi-word `"speech"` — the quote characters are kept and
//!   the whole run renders as a single block element in white (via the
//!   `md-speech` class) with emphasis nested inside it; single-word quotes
//!   (scare quotes like `"alive"`) stay plain text
//! - `- `/`* ` bullet lists and `1.`/`1)` ordered lists (flat, no nesting)
//!
//! Nothing has to be closed: a stream cut off mid-marker renders the
//! pending style through end-of-text instead of leaking raw markers.

/// Bold style bit for [`Segment::Styled`].
pub const BOLD: u8 = 1;
/// Italic style bit for [`Segment::Styled`].
pub const ITALIC: u8 = 2;

/// An inline element. Speech runs are containers — emphasis inside a quote
/// nests within the run, so the whole quoted phrase renders as one
/// block-level element instead of fragmenting into several.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
  /// A run of plain, bold or italic text — style bits [`BOLD`]/[`ITALIC`].
  Styled { style: u8, text: String },
  /// A double-quoted `"speech"` run — the quote characters are kept —
  /// rendered as one block (`md-speech`); children nest emphasis.
  Speech(Vec<Segment>),
}

/// CSS classes rendering the given emphasis style bits
/// (see [`Segment::Styled`]).
pub fn styled_classes(style: u8) -> &'static str {
  match (style & BOLD != 0, style & ITALIC != 0) {
    (true, true) => "font-bold italic",
    (true, false) => "font-bold",
    (false, true) => "italic",
    (false, false) => "",
  }
}

/// A block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
  Heading { level: u8, segments: Vec<Segment> },
  /// Contiguous `>` group — inner lines re-parsed as blocks.
  Quote(Vec<Block>),
  /// Contiguous list group of same-flavored items (flat — no nesting).
  List { ordered: bool, items: Vec<Vec<Segment>> },
  Paragraph(Vec<Segment>),
}

/// Parse text into blocks. Never fails; unterminated constructs render
/// as-if closed at end-of-text (streaming-friendly).
pub fn parse(text: &str) -> Vec<Block> {
  let mut blocks = Vec::new();
  let mut paragraph: Vec<&str> = Vec::new();
  let mut quote: Vec<&str> = Vec::new();
  let mut list: Option<(bool, Vec<String>)> = None;

  for line in text.lines() {
    let line = line.trim();
    if let Some(level) = heading_level(line) {
      flush_paragraph(&mut blocks, &mut paragraph);
      flush_list(&mut blocks, &mut list);
      flush_quote(&mut blocks, &mut quote);
      let content = line[level as usize..].trim_start();
      blocks.push(Block::Heading { level, segments: parse_inline(content) });
    } else if let Some(content) = line.strip_prefix('>') {
      flush_paragraph(&mut blocks, &mut paragraph);
      flush_list(&mut blocks, &mut list);
      quote.push(content.strip_prefix(' ').unwrap_or(content));
    } else if let Some((ordered, content)) = list_item(line) {
      flush_paragraph(&mut blocks, &mut paragraph);
      match &mut list {
        Some((known, items)) if *known == ordered => items.push(content.to_string()),
        _ => {
          flush_list(&mut blocks, &mut list);
          list = Some((ordered, vec![content.to_string()]));
        }
      }
    } else if line.is_empty() {
      flush_paragraph(&mut blocks, &mut paragraph);
      flush_list(&mut blocks, &mut list);
      flush_quote(&mut blocks, &mut quote);
    } else {
      flush_list(&mut blocks, &mut list);
      flush_quote(&mut blocks, &mut quote);
      paragraph.push(line);
    }
  }
  flush_paragraph(&mut blocks, &mut paragraph);
  flush_list(&mut blocks, &mut list);
  flush_quote(&mut blocks, &mut quote);
  blocks
}

/// Flush pending paragraph lines into a single [`Block::Paragraph`].
fn flush_paragraph(blocks: &mut Vec<Block>, paragraph: &mut Vec<&str>) {
  if paragraph.is_empty() {
    return;
  }
  let text = paragraph.join(" ");
  paragraph.clear();
  blocks.push(Block::Paragraph(parse_inline(&text)));
}

/// Flush a contiguous `>` group into a [`Block::Quote`].
fn flush_quote(blocks: &mut Vec<Block>, quote: &mut Vec<&str>) {
  if quote.is_empty() {
    return;
  }
  let inner = quote.join("\n");
  quote.clear();
  blocks.push(Block::Quote(parse(&inner)));
}

/// Flush the pending list group into a [`Block::List`].
fn flush_list(blocks: &mut Vec<Block>, list: &mut Option<(bool, Vec<String>)>) {
  let Some((ordered, items)) = list.take() else {
    return;
  };
  blocks.push(Block::List {
    ordered,
    items: items.iter().map(|item| parse_inline(item)).collect(),
  });
}

/// List marker of a line: `- ` or `* ` for unordered, digits followed by
/// `.` or `)` and a space for ordered. Returns whether the item is ordered
/// and its text.
fn list_item(line: &str) -> Option<(bool, &str)> {
  if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
    return Some((false, rest));
  }
  let digits = line.chars().take_while(|ch| ch.is_ascii_digit()).count();
  if digits == 0 || digits == line.len() {
    return None;
  }
  let marker = line.as_bytes()[digits];
  let rest = &line[digits + 1..];
  if (marker == b'.' || marker == b')') && rest.starts_with(' ') {
    return Some((true, rest[1..].trim_start()));
  }
  None
}

/// Heading level of a line: `#`..`######` followed by whitespace or EOL.
fn heading_level(line: &str) -> Option<u8> {
  let level = line.chars().take_while(|&ch| ch == '#').count();
  if level == 0 || level > 6 {
    return None;
  }
  match line[level..].chars().next() {
    None => Some(level as u8),
    Some(ch) if ch.is_whitespace() => Some(level as u8),
    Some(_) => None,
  }
}

/// Inline markers. Speech markers are kept in the output text, emphasis
/// markers are dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Marker {
  Bold,
  Italic,
  Speech,
}

impl Marker {
  /// Byte length of the literal marker.
  fn len(self) -> usize {
    match self {
      Self::Bold => 2,
      Self::Italic | Self::Speech => 1,
    }
  }

  fn at(text: &str, idx: usize) -> Option<Self> {
    let bytes = text.as_bytes();
    match bytes[idx] {
      b'"' => Some(Self::Speech),
      b'*' if bytes.get(idx + 1) == Some(&b'*') => Some(Self::Bold),
      b'*' => Some(Self::Italic),
      _ => None,
    }
  }
}

/// Leftmost inline marker in the text, if any.
fn next_marker(text: &str) -> Option<(usize, Marker)> {
  for (idx, byte) in text.bytes().enumerate() {
    if byte == b'*' || byte == b'"' {
      return Marker::at(text, idx).map(|marker| (idx, marker));
    }
  }
  None
}

/// Split inline text into segments. Double-quoted speech runs become
/// [`Segment::Speech`] containers — from the opening quote to the next
/// quote (or end-of-text while streaming) — with emphasis nested inside;
/// `**bold**` and `*italic*` toggle as before. Style state does not carry
/// across a quote boundary.
fn parse_inline(text: &str) -> Vec<Segment> {
  let mut segments = Vec::new();
  let mut plain = String::new();
  let mut style = 0u8;
  let mut rest = text;

  while let Some((idx, marker)) = next_marker(rest) {
    plain.push_str(&rest[..idx]);
    match marker {
      Marker::Speech => {
        // The quote characters are kept: the opening one belongs to the
        // following speech run, the closing one to the run it ends.
        let inner = &rest[idx + 1..];
        let (run, remainder) = match inner.split_once('"') {
          Some((run, remainder)) => (format!("\"{run}\""), remainder),
          None => (format!("\"{inner}"), ""),
        };
        if is_speech(&run) {
          if !plain.is_empty() {
            segments.push(Segment::Styled { style, text: std::mem::take(&mut plain) });
          }
          segments.push(Segment::Speech(parse_emphasis(&run)));
        } else {
          // Single-word quotes are scare quotes, not dialog — their text
          // stays plain content of the surrounding segment.
          plain.push_str(&run);
        }
        rest = remainder;
      }
      Marker::Bold | Marker::Italic => {
        if !plain.is_empty() {
          segments.push(Segment::Styled { style, text: std::mem::take(&mut plain) });
        }
        style ^= match marker {
          Marker::Bold => BOLD,
          _ => ITALIC,
        };
        rest = &rest[idx + marker.len()..];
      }
    }
  }

  plain.push_str(rest);
  if !plain.is_empty() {
    segments.push(Segment::Styled { style, text: plain });
  }
  segments
}

/// Whether a quoted run (including its quote characters) renders as
/// [`Segment::Speech`]: only multi-word content is dialog; single-word
/// `"scare quotes"` stay plain text.
fn is_speech(run: &str) -> bool {
  run.trim_matches('"').split_whitespace().count() > 1
}

/// Split speech-run text into emphasis segments. Quotes inside the run are
/// content; a marker left open at end-of-text keeps the style active for
/// the remainder (streaming-friendly).
fn parse_emphasis(text: &str) -> Vec<Segment> {
  let mut segments = Vec::new();
  let mut plain = String::new();
  let mut style = 0u8;
  let mut rest = text;

  while let Some((idx, marker)) = next_marker(rest) {
    plain.push_str(&rest[..idx]);
    match marker {
      Marker::Speech => {
        plain.push('"');
        rest = &rest[idx + 1..];
      }
      Marker::Bold | Marker::Italic => {
        if !plain.is_empty() {
          segments.push(Segment::Styled { style, text: std::mem::take(&mut plain) });
        }
        style ^= match marker {
          Marker::Bold => BOLD,
          _ => ITALIC,
        };
        rest = &rest[idx + marker.len()..];
      }
    }
  }

  plain.push_str(rest);
  if !plain.is_empty() {
    segments.push(Segment::Styled { style, text: plain });
  }
  segments
}

#[cfg(test)]
mod tests {
  use super::*;

  fn segs(text: &str) -> Vec<Segment> {
    parse_inline(text)
  }

  fn styled(style: u8, text: &str) -> Segment {
    Segment::Styled { style, text: text.to_string() }
  }

  fn plain(text: &str) -> Segment {
    styled(0, text)
  }

  fn speech(children: Vec<Segment>) -> Segment {
    Segment::Speech(children)
  }

  #[test]
  fn plain_text_is_one_segment() {
    assert_eq!(segs("hello world"), vec![plain("hello world")]);
  }

  #[test]
  fn bold_and_italic_drop_markers() {
    assert_eq!(
      segs("**bold** plain *italic*"),
      vec![styled(BOLD, "bold"), plain(" plain "), styled(ITALIC, "italic")]
    );
  }

  #[test]
  fn unclosed_bold_reaches_end() {
    assert_eq!(segs("stream **cut off"), vec![plain("stream "), styled(BOLD, "cut off")]);
  }

  #[test]
  fn bold_italic_nest() {
    assert_eq!(segs("***both***"), vec![styled(BOLD | ITALIC, "both")]);
  }

  #[test]
  fn speech_keeps_quotes_and_breaks_out() {
    assert_eq!(
      segs("He said \"listen closely\" and left"),
      vec![
        plain("He said "),
        speech(vec![plain("\"listen closely\"")]),
        plain(" and left"),
      ]
    );
  }

  #[test]
  fn unclosed_speech_reaches_end() {
    assert_eq!(segs("\"to be continued"), vec![speech(vec![plain("\"to be continued")])]);
  }

  #[test]
  fn emphasis_inside_speech_nests_within_the_run() {
    // A quote renders as ONE speech element with the emphasis nested inside
    // it — never as separate siblings, or it would fragment across lines.
    assert_eq!(
      segs("\"hello *there* friend\""),
      vec![speech(vec![
        plain("\"hello "),
        styled(ITALIC, "there"),
        plain(" friend\""),
      ])]
    );
    assert_eq!(
      segs("\"give me **everything** now\""),
      vec![speech(vec![
        plain("\"give me "),
        styled(BOLD, "everything"),
        plain(" now\""),
      ])]
    );
  }

  #[test]
  fn single_quotes_stay_plain() {
    assert_eq!(segs("it's 'fine'"), vec![plain("it's 'fine'")]);
  }

  #[test]
  fn single_word_quotes_stay_plain() {
    // Scare quotes are not dialog — no speech segment, text kept verbatim.
    assert_eq!(segs("keep \"alive\" at all costs"), vec![plain("keep \"alive\" at all costs")]);
    assert_eq!(segs("\"safety\""), vec![plain("\"safety\"")]);
  }

  #[test]
  fn multi_word_quotes_stay_speech() {
    assert_eq!(
      segs("keep \"alive and well\" at all costs"),
      vec![plain("keep "), speech(vec![plain("\"alive and well\"")]), plain(" at all costs")]
    );
  }

  #[test]
  fn heading_requires_space_or_eol() {
    assert!(matches!(parse("# real").as_slice(), [Block::Heading { level: 1, .. }]));
    assert!(matches!(parse("### deep").as_slice(), [Block::Heading { level: 3, .. }]));
    assert!(matches!(parse("#").as_slice(), [Block::Heading { level: 1, .. }]));
    assert!(matches!(parse("####### seven").as_slice(), [Block::Paragraph(_)]));
    assert!(matches!(parse("#tag stays").as_slice(), [Block::Paragraph(_)]));
  }

  #[test]
  fn headings_split_paragraphs() {
    let blocks = parse("## Heading\ntext");
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], Block::Heading { level: 2, segments } if segments.len() == 1));
    assert!(matches!(&blocks[1], Block::Paragraph(_)));
  }

  #[test]
  fn blank_lines_split_paragraphs_and_join_soft_lines() {
    let blocks = parse("one\ntwo\n\nthree");
    assert_eq!(blocks.len(), 2);
    let Block::Paragraph(segments) = &blocks[0] else { panic!("expected paragraph") };
    let Segment::Styled { text, .. } = &segments[0] else { panic!("expected styled segment") };
    assert_eq!(text, "one two");
  }

  #[test]
  fn blockquote_groups_contiguous_lines() {
    let blocks = parse("> line a\n> line b\n\nafter");
    assert_eq!(blocks.len(), 2);
    let Block::Quote(inner) = &blocks[0] else { panic!("expected quote") };
    assert_eq!(inner.len(), 1);
    assert!(matches!(&inner[0], Block::Paragraph(_)));
  }

  #[test]
  fn incomplete_stream_input_renders() {
    // Stream cut mid-heading, mid-marker and mid-speech — all render.
    let blocks = parse("## Partial tit");
    assert!(matches!(blocks.as_slice(), [Block::Heading { level: 2, .. }]));
    assert_eq!(segs("**still"), vec![styled(BOLD, "still")]);
    assert_eq!(segs("said \"hello there"), vec![plain("said "), speech(vec![plain("\"hello there")])]);
  }

  #[test]
  fn unordered_lists_group() {
    let blocks = parse("intro\n- one\n* two\n\noutro");
    assert_eq!(blocks.len(), 3);
    let Block::List { ordered, items } = &blocks[1] else { panic!("expected list") };
    assert!(!ordered);
    assert_eq!(items.len(), 2);
    let Segment::Styled { text, .. } = &items[0][0] else { panic!("expected styled segment") };
    assert_eq!(text, "one");
  }

  #[test]
  fn ordered_lists_group() {
    let blocks = parse("1. first\n2) second");
    assert_eq!(blocks.len(), 1);
    let Block::List { ordered, items } = &blocks[0] else { panic!("expected list") };
    assert!(ordered);
    assert_eq!(items.len(), 2);
  }

  #[test]
  fn ordered_marker_without_space_is_a_paragraph() {
    assert!(matches!(parse("3.14 is pi").as_slice(), [Block::Paragraph(_)]));
    assert!(matches!(parse("option 2)").as_slice(), [Block::Paragraph(_)]));
  }

  #[test]
  fn list_flavors_and_neighbors_split() {
    // Different flavors split into separate lists; plain text ends the list.
    let blocks = parse("- a\n1. b\nplain\n- c");
    assert_eq!(blocks.len(), 4);
    assert!(matches!(&blocks[0], Block::List { ordered: false, items } if items.len() == 1));
    assert!(matches!(&blocks[1], Block::List { ordered: true, items } if items.len() == 1));
    assert!(matches!(&blocks[2], Block::Paragraph(_)));
    assert!(matches!(&blocks[3], Block::List { ordered: false, items } if items.len() == 1));
  }

  #[test]
  fn list_items_parse_inline() {
    let blocks = parse("- **bold** item");
    let Block::List { items, .. } = &blocks[0] else { panic!("expected list") };
    assert_eq!(items[0], vec![styled(BOLD, "bold"), plain(" item")]);
  }
}
