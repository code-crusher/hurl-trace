/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use hurl_core::ast::{Placeholder, Template, TemplateElement};

use super::error::RunnerError;
use super::expr;
use super::variable::VariableSet;

/// Renders to string a `template` given a map of variables.
pub fn eval_template(template: &Template, variables: &VariableSet) -> Result<String, RunnerError> {
    let Template { elements, .. } = template;
    let mut value = String::new();
    for elem in elements {
        {
            let v = eval_template_element(elem, variables)?;
            value.push_str(v.as_str());
        }
    }
    Ok(value)
}

/// Renders to string a `template` given a map of variables, and returns for each output line
/// the 1-based absolute source line it originates from.
///
/// The returned vector has one entry per line of the rendered string, matching how lines are
/// split when diffing (`\n`, `\r` and `\r\n` are all line terminators). This is useful to map
/// a line in the rendered value back to its position in the source document, even when
/// variables introduce additional newlines.
pub fn eval_template_with_lines(
    template: &Template,
    variables: &VariableSet,
) -> Result<(String, Vec<usize>), RunnerError> {
    let Template {
        elements,
        source_info,
        ..
    } = template;
    let start_line = source_info.start.line;
    let mut value = String::new();
    // For each char of the rendered value, the 1-based source line it originates from.
    let mut char_sources = vec![];
    let mut current_line = start_line;
    for elem in elements {
        match elem {
            // Newlines from the template text advance the source line.
            TemplateElement::String { value: s, .. } => {
                for c in s.chars() {
                    char_sources.push(current_line);
                    value.push(c);
                    if c == '\n' {
                        current_line += 1;
                    }
                }
            }
            // Newlines rendered by a placeholder do not advance the source line: the
            // following output line still maps to the source line where the
            // placeholder is written.
            TemplateElement::Placeholder(_) => {
                let rendered = eval_template_element(elem, variables)?;
                for c in rendered.chars() {
                    char_sources.push(current_line);
                    value.push(c);
                }
            }
        }
    }

    let chars: Vec<char> = value.chars().collect();
    let mut lines = vec![if chars.is_empty() {
        start_line
    } else {
        char_sources[0]
    }];
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' || c == '\r' {
            let mut next = i + 1;
            if c == '\r' && next < chars.len() && chars[next] == '\n' {
                next += 1;
            }
            if next < chars.len() {
                lines.push(char_sources[next]);
            }
            i = next;
        } else {
            i += 1;
        }
    }
    Ok((value, lines))
}

fn eval_template_element(
    template_element: &TemplateElement,
    variables: &VariableSet,
) -> Result<String, RunnerError> {
    match template_element {
        TemplateElement::String { value, .. } => Ok(value.clone()),
        TemplateElement::Placeholder(Placeholder { expr, .. }) => expr::render(expr, variables),
    }
}

#[cfg(test)]
mod tests {
    use hurl_core::ast::{Expr, ExprKind, Template, TemplateElement, Variable, Whitespace};
    use hurl_core::ast::SourceInfo;
    use hurl_core::reader::Pos;
    use hurl_core::types::ToSource;

    use super::*;
    use crate::runner::{Number, RunnerErrorKind, Value};

    fn template_element_expression() -> TemplateElement {
        // {{name}}
        TemplateElement::Placeholder(Placeholder {
            space0: Whitespace {
                value: String::new(),
                source_info: SourceInfo::new(Pos::new(1, 3), Pos::new(1, 3)),
            },
            expr: Expr {
                kind: ExprKind::Variable(Variable {
                    name: "name".to_string(),
                    source_info: SourceInfo::new(Pos::new(1, 3), Pos::new(1, 7)),
                }),
                source_info: SourceInfo::new(Pos::new(1, 3), Pos::new(1, 7)),
            },
            space1: Whitespace {
                value: String::new(),
                source_info: SourceInfo::new(Pos::new(1, 3), Pos::new(1, 7)),
            },
        })
    }

    #[test]
    fn test_template_element() {
        let variables = VariableSet::new();
        assert_eq!(
            eval_template_element(
                &TemplateElement::String {
                    value: "World".to_string(),
                    source: "World".to_source(),
                },
                &variables
            )
            .unwrap(),
            "World".to_string()
        );

        let mut variables = VariableSet::new();
        variables.insert("name".to_string(), Value::String("World".to_string()));
        assert_eq!(
            eval_template_element(&template_element_expression(), &variables).unwrap(),
            "World".to_string()
        );
    }

    fn template(elements: Vec<TemplateElement>, start_line: usize) -> Template {
        Template::new(
            None,
            elements,
            SourceInfo::new(Pos::new(start_line, 1), Pos::new(start_line, 1)),
        )
    }

    #[test]
    fn test_eval_template_with_lines() {
        let template = template(
            vec![
                TemplateElement::String {
                    value: "line1\nline2".to_string(),
                    source: "line1\nline2".to_source(),
                },
                template_element_expression(),
                TemplateElement::String {
                    value: "line3\nline4\n".to_string(),
                    source: "line3\nline4\n".to_source(),
                },
            ],
            1,
        );
        let mut variables = VariableSet::new();
        variables.insert("name".to_string(), Value::String("!".to_string()));
        let (value, lines) = eval_template_with_lines(&template, &variables).unwrap();
        assert_eq!(value, "line1\nline2!line3\nline4\n");
        assert_eq!(lines, vec![1, 2, 3]);
    }

    #[test]
    fn test_eval_template_with_lines_placeholder_newline() {
        // The placeholder renders to a string containing a newline: the output line after
        // the newline maps to the source line the placeholder is written on, and the
        // following template text keeps advancing from there.
        let template = template(
            vec![
                TemplateElement::String {
                    value: "line1\nline2".to_string(),
                    source: "line1\nline2".to_source(),
                },
                template_element_expression(),
                TemplateElement::String {
                    value: "line3\nline4\n".to_string(),
                    source: "line3\nline4\n".to_source(),
                },
            ],
            10,
        );
        let mut variables = VariableSet::new();
        variables.insert("name".to_string(), Value::String("a\nb".to_string()));
        let (value, lines) = eval_template_with_lines(&template, &variables).unwrap();
        assert_eq!(value, "line1\nline2a\nbline3\nline4\n");
        // Lines: "line1" -> 10, "line2a" -> 11, "bline3" -> 11 (newline from the
        // placeholder keeps the placeholder's source line), "line4" -> 12 (template
        // text after the placeholder is on the next source line).
        assert_eq!(lines, vec![10, 11, 11, 12]);
    }

    #[test]
    fn test_template_element_error() {
        let mut variables = VariableSet::new();
        variables.insert(
            "name".to_string(),
            Value::List(vec![
                Value::Number(Number::Integer(1)),
                Value::Number(Number::Integer(2)),
            ]),
        );
        let error = eval_template_element(&template_element_expression(), &variables)
            .err()
            .unwrap();
        assert_eq!(
            error.source_info,
            SourceInfo::new(Pos::new(1, 3), Pos::new(1, 7))
        );
        assert_eq!(
            error.kind,
            RunnerErrorKind::UnrenderableExpression {
                value: "[1,2]".to_string()
            }
        );
    }
}
