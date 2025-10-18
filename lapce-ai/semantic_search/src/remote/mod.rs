// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

pub mod client;
pub mod db;
pub mod retry;
pub mod table;
pub mod util;

pub use client::{ClientConfig, RestfulLanceDbClient};
pub use db::RemoteDatabase;
pub use client::RetryConfig;
pub use table::RemoteTable;

pub const JSON_CONTENT_TYPE: &str = "application/json";
pub const ARROW_STREAM_CONTENT_TYPE: &str = "application/vnd.apache.arrow.stream";
pub const ARROW_FILE_CONTENT_TYPE: &str = "application/vnd.apache.arrow.file";

pub trait HeaderProvider {
    fn get_header(&self, name: &str) -> Option<String>;
}
