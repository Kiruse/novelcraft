//! Minimal markdown-like parser tuned for streaming agent output.
//!
//! Deliberately NOT CommonMark — only a small subset is supported:
//!
//! - paragraphs (blank-line separated; soft line breaks join with a space)
//! - `#`..`######` headings
//! - `>` blockquotes (contiguous groups; inner lines re-parsed as blocks)
//! - `**bold**` and `*italic*`
//! - double-quoted `"speech"` — the quote characters are kept and the run
//!   is rendered on its own line in white (via the `md-speech` class)
//!
//! Nothing has to be closed: a stream cut off mid-marker renders the
//! pending style through end-of-text instead of leaking raw markers.

/// Bold style bit for [`Segment::style`].
pub const BOLD: u8 = 1;
/// Italic style bit for [`Segment::style`].
pub const ITALIC: u8 = 2;
/// Double-quoted speech style bit for [`Segment::style`].
pub const SPEECH: u8 = 4;

/// A flat run of text sharing one style — inline constructs never nest.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
  pub style: u8,
  pub text: String,
}

impl Segment {
  /// CSS classes rendering this style combination.
  pub fn classes(&self) -> &'static str {
    let bold = self.style & BOLD != 0;
    let italic = self.style & ITALIC != 0;
    let speech = self.style & SPEECH != 0;
    match (bold, italic, speech) {
      (true, true, true) => "font-bold italic md-speech",
      (true, true, false) => "font-bold italic",
      (true, false, true) => "font-bold md-speech",
      (true, false, false) => "font-bold",
      (false, true, true) => "italic md-speech",
      (false, true, false) => "italic",
      (false, false, true) => "md-speech",
      (false, false, false) => "",
    }
  }
}

/// A block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
  Heading { level: u8, segments: Vec<Segment> },
  /// Contiguous `>` group — inner lines re-parsed as blocks.
  Quote(Vec<Block>),
  Paragraph(Vec<Segment>),
}

/// Parse text into blocks. Never fails; unterminated constructs render
/// as-if closed at end-of-text (streaming-friendly).
pub fn parse(text: &str) -> Vec<Block> {
  let mut blocks = Vec::new();
  let mut paragraph: Vec<&str> = Vec::new();
  let mut quote: Vec<&str> = Vec::new();

  for line in text.lines() {
    let line = line.trim();
    if let Some(level) = heading_level(line) {
      flush_paragraph(&mut blocks, &mut paragraph);
      flush_quote(&mut blocks, &mut quote);
      let content = line[level as usize..].trim_start();
      blocks.push(Block::Heading { level, segments: parse_inline(content) });
    } else if let Some(content) = line.strip_prefix('>') {
      flush_paragraph(&mut blocks, &mut paragraph);
      quote.push(content.strip_prefix(' ').unwrap_or(content));
    } else if line.is_empty() {
      flush_paragraph(&mut blocks, &mut paragraph);
      flush_quote(&mut blocks, &mut quote);
    } else {
      flush_quote(&mut blocks, &mut quote);
      paragraph.push(line);
    }
  }
  flush_paragraph(&mut blocks, &mut paragraph);
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
  fn bit(self) -> u8 {
    match self {
      Self::Bold => BOLD,
      Self::Italic => ITALIC,
      Self::Speech => SPEECH,
    }
  }

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

/// Split inline text into flat styled segments. Markers toggle their style
/// on/off; a marker left open at end-of-text keeps the style active for the
/// remainder (incomplete stream input still renders).
fn parse_inline(text: &str) -> Vec<Segment> {
  let mut segments = Vec::new();
  let mut plain = String::new();
  let mut style = 0u8;
  let mut rest = text;

  while let Some((idx, marker)) = next_marker(rest) {
    plain.push_str(&rest[..idx]);
    let bit = marker.bit();
    let closing = style & bit != 0;
    if marker == Marker::Speech {
      // Keep the quote characters in the rendered text: the opening one
      // belongs to the following speech run, the closing one to the
      // preceding speech run.
      if closing {
        plain.push('"');
        if !plain.is_empty() {
          segments.push(Segment { style, text: std::mem::take(&mut plain) });
        }
      } else {
        if !plain.is_empty() {
          segments.push(Segment { style, text: std::mem::take(&mut plain) });
        }
        plain.push('"');
      }
    } else if !plain.is_empty() {
      segments.push(Segment { style, text: std::mem::take(&mut plain) });
    }
    style ^= bit;
    rest = &rest[idx + marker.len()..];
  }

  plain.push_str(rest);
  if !plain.is_empty() {
    segments.push(Segment { style, text: plain });
  }
  segments
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Inline text as (style, content) pairs.
  fn segs(text: &str) -> Vec<(u8, String)> {
    parse_inline(text).into_iter().map(|s| (s.style, s.text)).collect()
  }

  #[test]
  fn plain_text_is_one_segment() {
    assert_eq!(segs("hello world"), vec![(0, "hello world".to_string())]);
  }

  #[test]
  fn bold_and_italic_drop_markers() {
    assert_eq!(
      segs("**bold** plain *italic*"),
      vec![(BOLD, "bold".to_string()), (0, " plain ".to_string()), (ITALIC, "italic".to_string())]
    );
  }

  #[test]
  fn unclosed_bold_reaches_end() {
    assert_eq!(
      segs("stream **cut off"),
      vec![(0, "stream ".to_string()), (BOLD, "cut off".to_string())]
    );
  }

  #[test]
  fn bold_italic_nest() {
    assert_eq!(segs("***both***"), vec![(BOLD | ITALIC, "both".to_string())]);
  }

  #[test]
  fn speech_keeps_quotes_and_breaks_out() {
    assert_eq!(
      segs("He said \"listen closely\" and left"),
      vec![
        (0, "He said ".to_string()),
        (SPEECH, "\"listen closely\"".to_string()),
        (0, " and left".to_string()),
      ]
    );
  }

  #[test]
  fn unclosed_speech_reaches_end() {
    assert_eq!(segs("\"to be continued"), vec![(SPEECH, "\"to be continued".to_string())]);
  }

  #[test]
  fn single_quotes_stay_plain() {
    assert_eq!(segs("it's 'fine'"), vec![(0, "it's 'fine'".to_string())]);
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
    assert_eq!(segments[0].text, "one two");
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
    assert_eq!(segs("**still"), vec![(BOLD, "still".to_string())]);
    assert_eq!(segs("said \"hi"), vec![(0, "said ".to_string()), (SPEECH, "\"hi".to_string())]);
  }
}
