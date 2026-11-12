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

/// Embeds the content of a Hurl file as a `&'static str`, at compile time.
///
/// The `path` is interpreted relatively to the file where this macro is invoked (like the
/// standard [`include_str!`] macro). It must be a literal, or an expression that expands to a
/// literal, such as [`concat!`] combined with [`env!`].
///
/// A leading [BOM](https://en.wikipedia.org/wiki/Byte_order_mark) is removed, as it is when the
/// `hurl` command line tool reads a Hurl file.
///
/// This macro is the counterpart of [`include_str!`] for Hurl files: it lets you keep your Hurl
/// requests in `.hurl` files (so they can still be run with the `hurl` command line tool) while
/// using them from Rust code through [`runner::run`](crate::runner::run). As the content is
/// embedded in the binary, there is no file lookup at runtime: a missing file is a compile error,
/// and a program can be run from any working directory.
///
/// # Example
///
/// ```no_run
/// use hurl::runner::{Input, RunnerOptionsBuilder, VariableSet};
/// use hurl::util::logger::LoggerOptionsBuilder;
///
/// // The Hurl template is embedded at compile time.
/// let content = hurl::include_hurl!("../tests/hello.hurl");
///
/// let runner_options = RunnerOptionsBuilder::new().build();
/// let logger_options = LoggerOptionsBuilder::new().color(false).build();
/// let variables = VariableSet::new();
/// let filename = Input::new("tests/hello.hurl");
///
/// let result = hurl::runner::run(
///     content,
///     Some(&filename),
///     &runner_options,
///     &variables,
///     &logger_options,
/// );
/// assert!(result.is_ok());
/// ```
///
/// A path relative to the crate root can be built with [`env!`]:
///
/// ```
/// let content = hurl::include_hurl!(concat!(
///     env!("CARGO_MANIFEST_DIR"),
///     "/tests/hello.hurl"
/// ));
/// assert!(content.contains("GET http://localhost:8000/hello"));
/// ```
#[macro_export]
macro_rules! include_hurl {
    ($path:expr) => {
        $crate::strip_bom(include_str!($path))
    };
}

/// Removes a leading UTF-8 BOM from `content`.
///
/// This function is used by the [`include_hurl!`] macro to embed Hurl files exactly as the `hurl`
/// command line tool reads them. It is public because macro expansion happens in the caller crate.
#[doc(hidden)]
pub const fn strip_bom(content: &'static str) -> &'static str {
    let bytes = content.as_bytes();
    if bytes.len() >= 3 && bytes[0] == 0xef && bytes[1] == 0xbb && bytes[2] == 0xbf {
        let (_, content) = content.split_at(3);
        content
    } else {
        content
    }
}

#[cfg(test)]
mod test {
    use super::strip_bom;
    use hurl_core::parser;

    /// The embedded content is the raw content of the Hurl file: it can be parsed, and run, as
    /// any other Hurl content.
    #[test]
    fn include_hurl_embeds_a_parsable_hurl_file() {
        let content = include_hurl!("../tests/hello.hurl");

        let hurl_file = parser::parse_hurl_file(content).unwrap();
        assert_eq!(hurl_file.entries.len(), 1);
        assert!(content.starts_with("GET http://localhost:8000/hello"));
    }

    /// The path can be computed from the crate root, which is useful when the Hurl file is not
    /// next to the Rust source file using it.
    #[test]
    fn include_hurl_accepts_a_computed_path() {
        let content = include_hurl!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/hello.hurl"));

        assert!(content.contains("HTTP 200"));
    }

    /// A Hurl file can start with a BOM: the macro removes it, as the `hurl` command line tool does.
    #[test]
    fn include_hurl_strips_a_leading_bom() {
        let content = include_hurl!("../tests/hello_with_bom.hurl");

        assert!(!content.starts_with('\u{feff}'));
        assert!(parser::parse_hurl_file(content).is_ok());
    }

    #[test]
    fn strip_bom_removes_a_leading_bom_only() {
        assert_eq!(strip_bom("hello"), "hello");
        assert_eq!(strip_bom("\u{feff}hello"), "hello");
        assert_eq!(strip_bom("\u{feff}"), "");
        assert_eq!(strip_bom("hel\u{feff}lo"), "hel\u{feff}lo");
    }
}
