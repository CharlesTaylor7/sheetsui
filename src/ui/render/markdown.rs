use std::collections::BTreeSet;

use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Widget,
};

use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq)]
pub struct Markdown {
    input: String,
    links: BTreeSet<String>,
    parsed_text: Option<Text<'static>>,
}

/// Define the different states a markdown parser can be in
#[derive(Debug, Clone, PartialEq)]
enum MarkdownState {
    Normal,
    Heading(pulldown_cmark::HeadingLevel),
    Strong,
    Emphasis,
    Code,
    List(ListState),
}

/// Track list state including nesting level and type
#[derive(Debug, Clone, PartialEq)]
struct ListState {
    list_type: ListType,
    nesting_level: usize,
    item_number: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum ListType {
    Ordered,
    Unordered,
}

impl Markdown {
    pub fn from_str(input: &str) -> Self {
        let mut me = Self {
            input: input.to_owned(),
            links: Default::default(),
            parsed_text: None,
        };
        me.parse();
        me
    }

    fn parse(&mut self) {
        let input = self.input.clone();

        let parser = pulldown_cmark::TextMergeStream::new(Parser::new(&input));

        let mut current_line = Line::default();
        let mut lines: Vec<Line> = Vec::new();
        let mut state_stack: Vec<MarkdownState> = vec![MarkdownState::Normal];

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match &tag {
                        Tag::Heading { level, .. } => {
                            if !current_line.spans.is_empty() {
                                lines.push(current_line);
                            }

                            // Add heading style based on level
                            let heading_style = match level {
                                pulldown_cmark::HeadingLevel::H1 => {
                                    Style::default().add_modifier(Modifier::BOLD)
                                }
                                pulldown_cmark::HeadingLevel::H2 => {
                                    Style::default().add_modifier(Modifier::ITALIC)
                                }
                                _ => Style::default().fg(Color::Blue),
                            };
                            current_line = Line::styled("", heading_style);
                            state_stack.push(MarkdownState::Heading(*level));
                        }
                        Tag::Paragraph => {
                            if !current_line.spans.is_empty() {
                                lines.push(current_line);
                                current_line = Line::default();
                            }
                        }
                        Tag::Strong => {
                            state_stack.push(MarkdownState::Strong);
                        }
                        Tag::Emphasis => {
                            state_stack.push(MarkdownState::Emphasis);
                        }
                        Tag::CodeBlock(_) => {
                            state_stack.push(MarkdownState::Code);
                        }
                        Tag::List(list_type) => {
                            if !current_line.spans.is_empty() {
                                lines.push(current_line);
                                current_line = Line::default();
                            }

                            // Determine list type and nesting level
                            let list_type = match list_type {
                                Some(_) => ListType::Ordered,
                                None => ListType::Unordered,
                            };

                            // Calculate nesting level based on existing lists in the stack
                            let nesting_level = state_stack
                                .iter()
                                .filter(|state| matches!(state, MarkdownState::List(_)))
                                .count();

                            state_stack.push(MarkdownState::List(ListState {
                                list_type,
                                nesting_level,
                                item_number: 0,
                            }));
                        }
                        Tag::Item => {
                            if !current_line.spans.is_empty() {
                                lines.push(current_line);
                                current_line = Line::default();
                            }

                            // Find the current list state and increment its item number
                            for state in state_stack.iter_mut().rev() {
                                if let MarkdownState::List(list_state) = state {
                                    list_state.item_number += 1;

                                    // Add appropriate indentation based on nesting level
                                    let indent = "  ".repeat(list_state.nesting_level);

                                    // Add appropriate marker based on list type
                                    let marker = match list_state.list_type {
                                        ListType::Unordered => "* ".to_string(),
                                        ListType::Ordered => {
                                            format!("{}. ", list_state.item_number)
                                        }
                                    };

                                    current_line
                                        .spans
                                        .push(Span::raw(format!("{}{}", indent, marker)));
                                    break;
                                }
                            }
                        }
                        Tag::Link {
                            link_type: _,
                            dest_url: _,
                            title: _,
                            id: _,
                        } => {
                            self.handle_link_tag(&tag);
                        }
                        Tag::BlockQuote(_) => todo!(),
                        Tag::Strikethrough => todo!(),
                        Tag::Superscript => todo!(),
                        Tag::Subscript => todo!(),
                        _ => {
                            // noop
                        }
                    }
                }
                Event::End(tag) => {
                    match tag {
                        TagEnd::Heading { .. } => {
                            lines.push(current_line);
                            lines.push(Line::default()); // Add empty line after heading
                            current_line = Line::default();
                            state_stack.pop();
                        }
                        TagEnd::Paragraph => {
                            lines.push(current_line);
                            lines.push(Line::default()); // Add empty line after paragraph
                            current_line = Line::default();
                        }
                        TagEnd::Strong => {
                            state_stack.pop();
                        }
                        TagEnd::Emphasis => {
                            state_stack.pop();
                        }
                        TagEnd::CodeBlock => {
                            state_stack.pop();
                        }
                        TagEnd::Item => {
                            // Push the current line to preserve the list item
                            if !current_line.spans.is_empty() {
                                lines.push(current_line);
                                current_line = Line::default();
                            }
                        }
                        TagEnd::List(_) => {
                            state_stack.pop();

                            // Only add an empty line if we're back to the root level
                            if state_stack
                                .iter()
                                .filter(|state| matches!(state, MarkdownState::List(_)))
                                .count()
                                == 0
                            {
                                //lines.push(Line::default()); // Add empty line after list
                            }
                        }
                        _ => {}
                    }
                }
                Event::InlineMath(text)
                | Event::Code(text)
                | Event::InlineHtml(text)
                | Event::DisplayMath(text)
                | Event::Html(text)
                | Event::Text(text) => {
                    let mut style = Style::default();

                    // Apply style based on current state
                    for state in state_stack.iter().rev() {
                        match state {
                            MarkdownState::Heading(_) => {
                                // Style already applied to the line
                                break;
                            }
                            MarkdownState::Strong => {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            MarkdownState::Emphasis => {
                                style = style.add_modifier(Modifier::ITALIC);
                            }
                            //MarkdownState::Code => {
                            //    style = style.fg(Color::Yellow);
                            //}
                            _ => {}
                        }
                    }

                    // Add the text with appropriate styling
                    current_line
                        .spans
                        .push(Span::styled(text.to_string(), style));
                }
                Event::SoftBreak => {
                    current_line.spans.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    lines.push(current_line);
                    current_line = Line::default();
                }
                Event::FootnoteReference(_) => {}
                Event::Rule => {}
                Event::TaskListMarker(_) => {}
            }
        }

        // Add any remaining content
        if !current_line.spans.is_empty() {
            lines.push(current_line);
        }

        self.parsed_text = Some(Text::from(lines));
    }

    fn handle_link_tag(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            } => {
                let dest = match link_type {
                    // [foo](bar)
                    LinkType::Inline => format!("({})", dest_url),
                    // [foo][bar]
                    LinkType::Reference => format!("[{}]", id),
                    // [foo]
                    LinkType::Shortcut => format!("[{}]", title),
                    // These are unsupported right now
                    LinkType::ReferenceUnknown => String::from("[unknown]"),
                    LinkType::Collapsed => String::from("[collapsed]"),
                    LinkType::CollapsedUnknown => String::from("[collapsed unknown]"),
                    LinkType::ShortcutUnknown => String::from("[shortcut unknown]"),
                    LinkType::Autolink => dest_url.to_string(),
                    LinkType::Email => dest_url.to_string(),
                    LinkType::WikiLink { has_pothole: _ } => String::from("[wiki]"),
                };
                self.links.insert(dest);
            }
            _ => { /* noop */ }
        }
    }

    pub fn handle_input(&self, code: KeyCode) -> Option<String> {
        let num = match code {
            KeyCode::Char('0') => 0,
            KeyCode::Char('1') => 1,
            KeyCode::Char('2') => 2,
            KeyCode::Char('3') => 3,
            KeyCode::Char('4') => 4,
            KeyCode::Char('5') => 5,
            KeyCode::Char('6') => 6,
            KeyCode::Char('7') => 7,
            KeyCode::Char('8') => 8,
            KeyCode::Char('9') => 9,
            _ => return None,
        };
        self.links.iter().nth(num).cloned()
    }

    pub fn get_text(&self) -> Text {
        if let Some(ref parsed) = self.parsed_text {
            parsed.clone()
        } else {
            Text::raw(&self.input)
        }
    }
}

impl Widget for Markdown {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if let Some(parsed) = self.parsed_text {
            parsed.render(area, buf);
        } else {
            let text = Text::raw(self.input);
            text.render(area, buf);
        }
    }
}
