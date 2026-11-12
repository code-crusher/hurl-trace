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
use hurl::runner::{Input, RunnerOptionsBuilder, VariableSet};
use hurl::util::logger::LoggerOptionsBuilder;
use hurl_core::parser;

/// A Hurl file can be kept as a template, runnable with the `hurl` CLI, and embedded in a Rust
/// program at compile time.
#[test]
fn hurl_file_can_be_embedded_and_parsed() {
    let content = hurl::include_hurl!("hello.hurl");

    let hurl_file = parser::parse_hurl_file(content).unwrap();
    assert_eq!(hurl_file.entries.len(), 1);
}

/// The embedded content is a plain `&'static str` that can be passed to [`hurl::runner::run`].
/// `runner::run` only returns an error when the content can not be parsed: connection errors are
/// reported in the returned [`hurl::runner::HurlResult`], so this test does not require an HTTP
/// server to be available.
#[test]
fn hurl_file_can_be_run_from_rust_code() {
    let content = hurl::include_hurl!("hello.hurl");
    let filename = Input::new("tests/hello.hurl");
    let runner_options = RunnerOptionsBuilder::new().build();
    let variables = VariableSet::new();
    let logger_options = LoggerOptionsBuilder::new().color(false).build();

    let result = hurl::runner::run(
        content,
        Some(&filename),
        &runner_options,
        &variables,
        &logger_options,
    );
    assert!(result.is_ok());
}

/// A path relative to the crate root can be used, which is handy when the Hurl file lives outside
/// of the Rust source tree.
#[test]
fn hurl_file_path_can_be_built_from_crate_root() {
    let content = hurl::include_hurl!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/hello.hurl"));

    assert!(content.contains("GET http://localhost:8000/hello"));
    assert!(content.contains("HTTP 200"));
}
