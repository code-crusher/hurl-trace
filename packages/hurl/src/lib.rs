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

//! This crate provides a function to run a Hurl formatted content.
//! Hurl uses a plain text format to run and tests HTTP requests. The fully documented
//! format is available at <https://hurl.dev>
//!
//! A Hurl sample:
//! ```hurl
//! # Get home:
//! GET https://example.org
//! HTTP 200
//! [Captures]
//! csrf_token: xpath "string(//meta[@name='_csrf_token']/@content)"
//!
//!
//! # Do login!
//! POST https://example.org/login?user=toto&password=1234
//! X-CSRF-TOKEN: {{csrf_token}}
//! HTTP 302
//! ```
//!
//! The main function of this crate is [`runner::run`].
//!
//! # Using Hurl files from Rust code
//!
//! Hurl requests can be kept in `.hurl` files, so that they are runnable with the `hurl` command
//! line tool, and embedded in a Rust program at compile time with the [`include_hurl!`] macro:
//!
//! ```no_run
//! use hurl::runner::{Input, RunnerOptionsBuilder, VariableSet};
//! use hurl::util::logger::LoggerOptionsBuilder;
//!
//! // The Hurl file is embedded in the binary at compile time.
//! let content = hurl::include_hurl!("../tests/hello.hurl");
//! let filename = Input::new("tests/hello.hurl");
//!
//! let runner_options = RunnerOptionsBuilder::new().build();
//! let logger_options = LoggerOptionsBuilder::new().color(false).build();
//! let variables = VariableSet::new();
//!
//! let result = hurl::runner::run(
//!     content,
//!     Some(&filename),
//!     &runner_options,
//!     &variables,
//!     &logger_options,
//! );
//! assert!(result.is_ok());
//! ```
//!
//! This crate works on Windows, macOS and Linux.

mod html;
pub mod http;
mod json;
mod jsonpath;
mod macros;
#[doc(hidden)]
pub use macros::strip_bom;
pub mod output;
#[doc(hidden)]
pub mod parallel;
pub mod pretty;
pub mod report;
pub mod runner;
pub mod util;
