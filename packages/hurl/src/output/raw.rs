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
use crate::pretty::PrettyMode;
use crate::runner::{HurlResult, Output};
use crate::util::term::Stdout;

use super::OutputError;

/// Writes the `hurl_result` last response to the file `filename_out`.
///
/// When `include_headers` is true, the last HTTP response headers are written before the body response.
/// When `filename_out` is `None`, standard output is used.
/// When `append` is true, any existing file will be appended instead of being truncated.
/// The body can pe prettified base on `pretty` value.
pub fn write_last_body(
    hurl_result: &HurlResult,
    include_headers: bool,
    color: bool,
    pretty: PrettyMode,
    filename_out: Option<&Output>,
    stdout: &mut Stdout,
    append: bool,
) -> Result<(), OutputError> {
    // Get the last call of the Hurl result.
    let Some(last_entry) = &hurl_result.entries.last() else {
        return Ok(());
    };
    let source_info = last_entry.source_info;
    last_entry.write_response(
        filename_out,
        stdout,
        include_headers,
        color,
        pretty,
        append,
        source_info,
    )?;
    Ok(())
}

/// Writes the `hurl_result` last response headers to the file `filename_out`, without the body
/// response.
///
/// When `filename_out` is `None`, standard output is used.
/// When `append` is true, any existing file will be appended instead of being truncated.
pub fn write_last_headers(
    hurl_result: &HurlResult,
    color: bool,
    filename_out: Option<&Output>,
    stdout: &mut Stdout,
    append: bool,
) -> Result<(), OutputError> {
    // Get the last call of the Hurl result.
    let Some(last_entry) = &hurl_result.entries.last() else {
        return Ok(());
    };
    let source_info = last_entry.source_info;
    last_entry.write_headers(filename_out, stdout, color, append, source_info)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::time::Duration;

    use crate::http::{Call, Header, HeaderVec, HttpVersion, Request, Response, Url};
    use crate::output;
    use crate::output::write_last_body;
    use crate::pretty::PrettyMode;
    use crate::runner::{EntryResult, HurlResult, Output};
    use crate::util::term::{Stdout, WriteMode};
    use hurl_core::ast::SourceInfo;
    use hurl_core::reader::Pos;
    use hurl_core::types::Index;

    fn default_response() -> Response {
        Response {
            version: HttpVersion::Http10,
            status: 200,
            headers: HeaderVec::new(),
            body: vec![],
            duration: Default::default(),
            url: Url::from_str("http://localhost").unwrap(),
            certificate: None,
            ip_addr: Default::default(),
        }
    }

    fn hurl_result_json() -> HurlResult {
        let mut headers = HeaderVec::new();
        headers.push(Header::new("x-foo", "xxx"));
        headers.push(Header::new("x-bar", "yyy0"));
        headers.push(Header::new("x-bar", "yyy1"));
        headers.push(Header::new("x-bar", "yyy2"));
        headers.push(Header::new("x-baz", "zzz"));

        HurlResult {
            entries: vec![
                EntryResult {
                    entry_index: Index::new(1),
                    source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
                    calls: vec![Call {
                        request: Request {
                            url: Url::from_str("https://foo.com").unwrap(),
                            method: "GET".to_string(),
                            headers: HeaderVec::new(),
                            body: vec![],
                        },
                        response: default_response(),
                        timings: Default::default(),
                    }],
                    ..Default::default()
                },
                EntryResult {
                    entry_index: Index::new(2),
                    source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
                    calls: vec![Call {
                        request: Request {
                            url: Url::from_str("https://bar.com").unwrap(),
                            method: "GET".to_string(),
                            headers: HeaderVec::new(),
                            body: vec![],
                        },
                        response: default_response(),
                        timings: Default::default(),
                    }],
                    ..Default::default()
                },
                EntryResult {
                    entry_index: Index::new(3),
                    source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
                    calls: vec![Call {
                        request: Request {
                            url: Url::from_str("https://baz.com").unwrap(),
                            method: "GET".to_string(),
                            headers: HeaderVec::new(),
                            body: vec![],
                        },
                        response: Response {
                            version: HttpVersion::Http3,
                            status: 204,
                            headers,
                            body: b"{\"say\": \"Hello World!\"}".into(),
                            duration: Default::default(),
                            url: Url::from_str("https://baz.com").unwrap(),
                            certificate: None,
                            ip_addr: Default::default(),
                        },
                        timings: Default::default(),
                    }],
                    ..Default::default()
                },
            ],
            duration: Duration::from_millis(100),
            success: true,
            ..Default::default()
        }
    }

    #[test]
    fn write_last_body_with_headers() {
        let result = hurl_result_json();
        let include_header = true;
        let color = false;
        let pretty = PrettyMode::None;
        let output = Some(Output::Stdout);
        let mut stdout = Stdout::new(WriteMode::Buffered);

        write_last_body(
            &result,
            include_header,
            color,
            pretty,
            output.as_ref(),
            &mut stdout,
            true,
        )
        .unwrap();
        let stdout = String::from_utf8(stdout.buffer().to_vec()).unwrap();
        assert_eq!(
            stdout,
            "HTTP/3 204\n\
             x-foo: xxx\n\
             x-bar: yyy0\n\
             x-bar: yyy1\n\
             x-bar: yyy2\n\
             x-baz: zzz\n\
             \n\
             {\"say\": \"Hello World!\"}"
        );
    }

    #[test]
    fn write_last_headers() {
        let result = hurl_result_json();
        let color = false;
        let output = Some(Output::Stdout);
        let mut stdout = Stdout::new(WriteMode::Buffered);

        output::write_last_headers(&result, color, output.as_ref(), &mut stdout, true).unwrap();
        let stdout = String::from_utf8(stdout.buffer().to_vec()).unwrap();
        assert_eq!(
            stdout,
            "HTTP/3 204\n\
             x-foo: xxx\n\
             x-bar: yyy0\n\
             x-bar: yyy1\n\
             x-bar: yyy2\n\
             x-baz: zzz\n\
             \n"
        );
    }
}
