//! Reduces a source file to the bytes and lines that carry program text.
//!
//! Comments are stripped using the language's own [`CommentSyntax`] from
//! `entl-codebase`, so the measurement follows Entl's language profiles rather
//! than a second, drifting copy of them. String literals survive: they are
//! program content, and a language that needs more of them is more verbose.

use entl_codebase::CommentSyntax;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// Lines that hold program text after comments and blank lines are removed.
    pub lines: u32,
    /// Bytes of those lines, with leading and trailing whitespace removed so
    /// that indentation style does not register as verbosity.
    pub bytes: u32,
}

impl Measurement {
    pub fn is_empty(&self) -> bool {
        self.lines == 0 || self.bytes == 0
    }
}

pub fn measure(source: &str, syntax: &CommentSyntax) -> Measurement {
    let code = strip_comments(source, syntax);
    let mut lines = 0;
    let mut bytes = 0;
    for line in code.lines() {
        let line = line.trim();
        if !line.is_empty() {
            lines += 1;
            bytes += line.len() as u32;
        }
    }
    Measurement { lines, bytes }
}

enum State {
    Code,
    LineComment,
    BlockComment(&'static str),
    Quoted(char),
    MultiQuoted(&'static str),
}

/// Removes comments while preserving line structure, so that code separated by
/// a block comment does not merge into one line.
fn strip_comments(source: &str, syntax: &CommentSyntax) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut output = String::with_capacity(source.len());
    let mut state = State::Code;
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];
        match state {
            State::Code => {
                if character == '\n' {
                    output.push('\n');
                    index += 1;
                } else if let Some(delimiter) = matches_any(&characters, index, syntax.multi_quotes)
                {
                    output.push_str(delimiter);
                    index += delimiter.chars().count();
                    state = State::MultiQuoted(delimiter);
                } else if let Some((open, close)) = matches_block(&characters, index, syntax.block)
                {
                    index += open.chars().count();
                    state = State::BlockComment(close);
                } else if let Some(marker) = matches_any(&characters, index, syntax.line) {
                    index += marker.chars().count();
                    state = State::LineComment;
                } else if syntax.quotes.contains(&character) {
                    output.push(character);
                    index += 1;
                    state = State::Quoted(character);
                } else {
                    output.push(character);
                    index += 1;
                }
            }
            State::LineComment => {
                if character == '\n' {
                    output.push('\n');
                    state = State::Code;
                }
                index += 1;
            }
            State::BlockComment(close) => {
                if let Some(marker) = matches_any(&characters, index, &[close]) {
                    index += marker.chars().count();
                    state = State::Code;
                } else {
                    if character == '\n' {
                        output.push('\n');
                    }
                    index += 1;
                }
            }
            State::Quoted(quote) => {
                output.push(character);
                index += 1;
                if character == '\\' && index < characters.len() {
                    output.push(characters[index]);
                    index += 1;
                } else if character == quote || character == '\n' {
                    // A newline means the quote was not a string opener at all
                    // (a Rust lifetime, an English apostrophe in shell). Recover
                    // rather than swallowing the rest of the file.
                    state = State::Code;
                }
            }
            State::MultiQuoted(delimiter) => {
                if let Some(marker) = matches_any(&characters, index, &[delimiter]) {
                    output.push_str(marker);
                    index += marker.chars().count();
                    state = State::Code;
                } else {
                    output.push(character);
                    index += 1;
                }
            }
        }
    }

    output
}

/// Returns the longest marker in `markers` that starts at `index`, so that
/// `"""` wins over `"` and `///` over `//`.
fn matches_any(
    characters: &[char],
    index: usize,
    markers: &[&'static str],
) -> Option<&'static str> {
    markers
        .iter()
        .filter(|marker| starts_with(characters, index, marker))
        .max_by_key(|marker| marker.chars().count())
        .copied()
}

fn matches_block(
    characters: &[char],
    index: usize,
    blocks: &[(&'static str, &'static str)],
) -> Option<(&'static str, &'static str)> {
    blocks
        .iter()
        .filter(|(open, _)| starts_with(characters, index, open))
        .max_by_key(|(open, _)| open.chars().count())
        .copied()
}

fn starts_with(characters: &[char], index: usize, marker: &str) -> bool {
    marker
        .chars()
        .enumerate()
        .all(|(offset, expected)| characters.get(index + offset) == Some(&expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use entl_codebase::comment_syntax;

    fn syntax(language: &str) -> &'static CommentSyntax {
        comment_syntax(language).expect("language has comment syntax")
    }

    #[test]
    fn drops_line_comments_and_blank_lines() {
        let measured = measure("// header\n\nint main() {}\n", syntax("c"));
        assert_eq!(
            measured,
            Measurement {
                lines: 1,
                bytes: 13
            }
        );
    }

    #[test]
    fn keeps_code_split_across_a_block_comment() {
        let measured = measure("a;\n/* note\n   more */\nb;\n", syntax("c"));
        assert_eq!(measured, Measurement { lines: 2, bytes: 4 });
    }

    #[test]
    fn ignores_comment_markers_inside_strings() {
        let measured = measure("print(\"# not a comment\")\n", syntax("python"));
        assert_eq!(
            measured,
            Measurement {
                lines: 1,
                bytes: 24
            }
        );
    }

    #[test]
    fn treats_python_triple_quotes_as_content() {
        let measured = measure("x = \"\"\"a\nb\"\"\"\n", syntax("python"));
        assert_eq!(
            measured,
            Measurement {
                lines: 2,
                bytes: 12
            }
        );
    }

    #[test]
    fn survives_rust_lifetimes() {
        let measured = measure("fn f<'a>(x: &'a str) {}\nlet y = 1;\n", syntax("rust"));
        assert_eq!(measured.lines, 2);
    }

    #[test]
    fn strips_indentation_but_not_inner_spacing() {
        let measured = measure("        if x == 1:\n", syntax("python"));
        assert_eq!(
            measured,
            Measurement {
                lines: 1,
                bytes: 10
            }
        );
    }
}
