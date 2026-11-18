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

use super::error::{RunnerError, RunnerErrorKind};
use super::expr;
use super::value::Value;
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

/// Renders to bytes a `template` given a map of variables.
///
/// Contrary to [`eval_template`], `bytes` captures are rendered as raw bytes: they can be
/// re-injected without any loss in binary contexts like multipart form data params or bodies.
/// Renderable values (like strings or numbers) are rendered as in [`eval_template`].
///
/// ```hurl
/// GET https://example.org/backup
/// HTTP 200
/// [Captures]
/// data: bytes
///
/// POST https://example.org/restore
/// [Multipart]
/// backup: {{data}}
/// ```
pub fn eval_template_bytes(
    template: &Template,
    variables: &VariableSet,
) -> Result<Vec<u8>, RunnerError> {
    let Template { elements, .. } = template;
    let mut value = Vec::new();
    for elem in elements {
        match elem {
            TemplateElement::String { value: s, .. } => value.extend_from_slice(s.as_bytes()),
            TemplateElement::Placeholder(Placeholder { expr, .. }) => {
                let v = expr::eval(expr, variables)?;
                // A `bytes` value is rendered as raw bytes, so binary data can be re-injected
                // without any loss. Other values are rendered as text.
                let bytes = match v {
                    Value::Bytes(bytes) => bytes,
                    v => match v.render() {
                        Some(s) => s.into_bytes(),
                        None => {
                            let kind = RunnerErrorKind::UnrenderableExpression {
                                value: v.to_string(),
                            };
                            return Err(RunnerError::new(expr.source_info, kind, false));
                        }
                    },
                };
                value.extend_from_slice(bytes.as_slice());
            }
        }
    }
    Ok(value)
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
    use hurl_core::ast::{Expr, ExprKind, SourceInfo, Variable, Whitespace};
    use hurl_core::reader::Pos;
    use hurl_core::types::ToSource;

    use super::*;
    use crate::runner::{Number, RunnerErrorKind, Value};

    fn template_element_bytes_expression() -> TemplateElement {
        // {{data}}
        TemplateElement::Placeholder(Placeholder {
            space0: Whitespace {
                value: String::new(),
                source_info: SourceInfo::new(Pos::new(1, 3), Pos::new(1, 3)),
            },
            expr: Expr {
                kind: ExprKind::Variable(Variable {
                    name: "data".to_string(),
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

    #[test]
    fn test_template_bytes() {
        // The template "Hello {{data}}"
        let mut variables = VariableSet::new();
        variables.insert(
            "data".to_string(),
            Value::Bytes(b"\n<binary \xff\xfe>".to_vec()),
        );
        let template = Template::new(
            None,
            vec![
                TemplateElement::String {
                    value: "Hello ".to_string(),
                    source: "Hello ".to_source(),
                },
                template_element_bytes_expression(),
            ],
            SourceInfo::new(Pos::new(1, 1), Pos::new(1, 20)),
        );
        assert_eq!(
            eval_template_bytes(&template, &variables).unwrap(),
            b"Hello \n<binary \xff\xfe>".to_vec()
        );
    }

    #[test]
    fn test_template_bytes_renderable_value() {
        // The template "{{status}}"
        let mut variables = VariableSet::new();
        variables.insert("status".to_string(), Value::Number(Number::Integer(200)));
        let template = Template::new(
            None,
            vec![TemplateElement::Placeholder(Placeholder {
                space0: Whitespace {
                    value: String::new(),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                },
                expr: Expr {
                    kind: ExprKind::Variable(Variable {
                        name: "status".to_string(),
                        source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                    }),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                },
                space1: Whitespace {
                    value: String::new(),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                },
            })],
            SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
        );
        assert_eq!(
            eval_template_bytes(&template, &variables).unwrap(),
            b"200".to_vec()
        );
    }

    #[test]
    fn test_template_bytes_error() {
        // The template "{{values}}"
        let mut variables = VariableSet::new();
        variables.insert(
            "values".to_string(),
            Value::List(vec![
                Value::Number(Number::Integer(1)),
                Value::Number(Number::Integer(2)),
            ]),
        );
        let template = Template::new(
            None,
            vec![TemplateElement::Placeholder(Placeholder {
                space0: Whitespace {
                    value: String::new(),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                },
                expr: Expr {
                    kind: ExprKind::Variable(Variable {
                        name: "values".to_string(),
                        source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                    }),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                },
                space1: Whitespace {
                    value: String::new(),
                    source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
                },
            })],
            SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9)),
        );
        let error = eval_template_bytes(&template, &variables).err().unwrap();
        assert_eq!(
            error.source_info,
            SourceInfo::new(Pos::new(1, 1), Pos::new(1, 9))
        );
        assert_eq!(
            error.kind,
            RunnerErrorKind::UnrenderableExpression {
                value: "[1,2]".to_string()
            }
        );
    }
}
