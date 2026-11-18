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
use std::path::PathBuf;

use hurl_core::ast::{Base64, Body, Bytes, File, Hex, Template};

use crate::http;
use crate::util::path::ContextDir;

use super::error::{RunnerError, RunnerErrorKind};
use super::json::eval_json_value;
use super::multiline::eval_multiline_bytes;
use super::template::{eval_template, eval_template_bytes};
use super::variable::VariableSet;

pub fn eval_body(
    body: &Body,
    variables: &VariableSet,
    context_dir: &ContextDir,
) -> Result<http::Body, RunnerError> {
    eval_bytes(&body.value, variables, context_dir)
}

pub fn eval_bytes(
    bytes: &Bytes,
    variables: &VariableSet,
    context_dir: &ContextDir,
) -> Result<http::Body, RunnerError> {
    match bytes {
        Bytes::OnelineString(value) => {
            let value = eval_template_bytes(value, variables)?;
            eval_body_text(value)
        }
        Bytes::MultilineString(value) => {
            let value = eval_multiline_bytes(value, variables)?;
            eval_body_text(value)
        }
        Bytes::Xml(value) => Ok(http::Body::Text(value.clone())),
        Bytes::Json(value) => {
            let value = eval_json_value(value, variables, true)?;
            Ok(http::Body::Text(value))
        }
        Bytes::Base64(Base64 { value, .. }) => Ok(http::Body::Binary(value.clone())),
        Bytes::Hex(Hex { value, .. }) => Ok(http::Body::Binary(value.clone())),
        Bytes::File(File { filename, .. }) => {
            let value = eval_file(filename, variables, context_dir)?;
            let filename = eval_template(filename, variables)?;
            Ok(http::Body::File(value, filename))
        }
    }
}

/// Returns the body for a text template which may embed `bytes` captures.
///
/// The body remains textual when the rendered value is an UTF-8 string; if the value contains
/// binary bytes (from a `bytes` capture), a binary body is returned so no data is lost.
fn eval_body_text(value: Vec<u8>) -> Result<http::Body, RunnerError> {
    match String::from_utf8(value) {
        Ok(value) => Ok(http::Body::Text(value)),
        Err(e) => Ok(http::Body::Binary(e.into_bytes())),
    }
}

pub fn eval_file(
    filename: &Template,
    variables: &VariableSet,
    context_dir: &ContextDir,
) -> Result<Vec<u8>, RunnerError> {
    let file = eval_template(filename, variables)?;
    // In order not to leak any private date, we check that the user provided file
    // is a child of the context directory.
    let path = PathBuf::from(file);
    if !context_dir.is_access_allowed(&path) {
        let kind = RunnerErrorKind::UnauthorizedFileAccess { path };
        return Err(RunnerError::new(filename.source_info, kind, false));
    }
    let resolved_file = context_dir.resolved_path(&path);
    match std::fs::read(resolved_file) {
        Ok(value) => Ok(value),
        Err(_) => {
            let kind = RunnerErrorKind::FileReadAccess { path };
            Err(RunnerError::new(filename.source_info, kind, false))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use hurl_core::ast::{
        Expr, ExprKind, Placeholder, SourceInfo, TemplateElement, Variable, Whitespace,
    };
    use hurl_core::reader::Pos;
    use hurl_core::types::ToSource;

    use super::*;
    use crate::runner::Value;

    #[test]
    fn test_body_file() {
        // file, data.bin;
        let whitespace = Whitespace {
            value: String::from(" "),
            source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
        };

        let bytes = Bytes::File(File {
            space0: whitespace.clone(),
            filename: Template::new(
                None,
                vec![TemplateElement::String {
                    value: "tests/data.bin".to_string(),
                    source: "tests/data.bin".to_source(),
                }],
                SourceInfo::new(Pos::new(1, 7), Pos::new(1, 15)),
            ),
            space1: whitespace,
        });

        let variables = VariableSet::new();
        let current_dir = Path::new("/my_home");
        let file_root = Path::new("");
        let context_dir = ContextDir::new(current_dir, file_root);
        assert_eq!(
            eval_bytes(&bytes, &variables, &context_dir).unwrap(),
            http::Body::File(b"Hello World!".to_vec(), "tests/data.bin".to_string())
        );
    }

    #[test]
    fn test_body_file_error() {
        // file, data.bin;
        let whitespace = Whitespace {
            value: String::from(" "),
            source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
        };

        let bytes = Bytes::File(File {
            space0: whitespace.clone(),
            filename: Template::new(
                None,
                vec![TemplateElement::String {
                    value: "data.bin".to_string(),
                    source: "data.bin".to_source(),
                }],
                SourceInfo::new(Pos::new(1, 7), Pos::new(1, 15)),
            ),
            space1: whitespace,
        });

        let variables = VariableSet::new();

        let current_dir = Path::new("/my_home");
        let file_root = Path::new("file_root");
        let context_dir = ContextDir::new(current_dir, file_root);
        let error = eval_bytes(&bytes, &variables, &context_dir).err().unwrap();
        assert_eq!(
            error.kind,
            RunnerErrorKind::FileReadAccess {
                path: PathBuf::from("data.bin")
            }
        );
        assert_eq!(
            error.source_info,
            SourceInfo::new(Pos::new(1, 7), Pos::new(1, 15))
        );
    }

    /// A text body which embeds a `bytes` capture is evaluated to a binary body, so binary
    /// data is not lost (see <https://github.com/Orange-OpenSource/hurl/issues/3752>).
    #[test]
    fn test_body_text_with_bytes() {
        let mut variables = VariableSet::new();
        variables.insert("data".to_string(), Value::Bytes(b"\x00\x01\x02\xff".to_vec()));

        let bytes = Bytes::OnelineString(Template::new(
            Some('`'),
            vec![
                TemplateElement::String {
                    value: "prefix".to_string(),
                    source: "prefix".to_source(),
                },
                TemplateElement::Placeholder(Placeholder {
                    space0: Whitespace {
                        value: String::new(),
                        source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                    },
                    expr: Expr {
                        kind: ExprKind::Variable(Variable {
                            name: "data".to_string(),
                            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                        }),
                        source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                    },
                    space1: Whitespace {
                        value: String::new(),
                        source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
                    },
                }),
            ],
            SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
        ));

        assert_eq!(
            eval_bytes(&bytes, &variables, &ContextDir::default()).unwrap(),
            http::Body::Binary(b"prefix\x00\x01\x02\xff".to_vec())
        );

        // The same body, with a renderable value, remains textual.
        variables.insert(
            "data".to_string(),
            Value::String("value".to_string()),
        );
        assert_eq!(
            eval_bytes(&bytes, &variables, &ContextDir::default()).unwrap(),
            http::Body::Text("prefixvalue".to_string())
        );
    }
}
