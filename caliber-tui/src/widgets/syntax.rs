//! Simple syntax highlighting for CALIBER DSL.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct SyntaxHighlighter {
    pub keyword_color: Color,
    pub memory_color: Color,
    pub field_color: Color,
    pub string_color: Color,
    pub number_color: Color,
}

impl SyntaxHighlighter {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect, content: &str) {
        let text = self.highlight(content);
        let paragraph = Paragraph::new(text)
            .block(Block::default().title("DSL").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    pub fn highlight(&self, content: &str) -> Text<'static> {
        let mut spans: Vec<Span> = Vec::new();
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.peek().copied() {
            if ch == '"' {
                spans.push(Span::styled("\"", Style::default().fg(self.string_color)));
                chars.next();
                let mut value = String::new();
                for next in chars.by_ref() {
                    if next == '"' {
                        break;
                    }
                    value.push(next);
                }
                spans.push(Span::styled(value, Style::default().fg(self.string_color)));
                spans.push(Span::styled("\"", Style::default().fg(self.string_color)));
                continue;
            }

            if ch.is_ascii_digit() {
                let mut number = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_digit() || next == '.' {
                        number.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                spans.push(Span::styled(number, Style::default().fg(self.number_color)));
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let mut word = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_alphanumeric() || next == '_' {
                        word.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let lower = word.to_ascii_lowercase();
                let style = if is_keyword(&lower) {
                    Style::default().fg(self.keyword_color)
                } else if is_memory_type(&lower) {
                    Style::default().fg(self.memory_color)
                } else if is_field_type(&lower) {
                    Style::default().fg(self.field_color)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(word, style));
                continue;
            }

            spans.push(Span::raw(ch.to_string()));
            chars.next();
        }

        Text::from(Line::from(spans))
    }
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "caliber" | "memory" | "policy" | "adapter" | "inject"
    )
}

fn is_memory_type(value: &str) -> bool {
    matches!(
        value,
        "ephemeral" | "working" | "episodic" | "semantic" | "procedural" | "meta"
    )
}

fn is_field_type(value: &str) -> bool {
    matches!(
        value,
        "uuid" | "text" | "int" | "float" | "bool" | "timestamp" | "json" | "embedding"
    )
}
