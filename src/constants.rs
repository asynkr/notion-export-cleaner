use std::sync::LazyLock;

use indicatif::ProgressStyle;

pub const NOTION_LINK_MARKER: &str = "notion.so";

pub static PROGRESS_BAR_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| ProgressStyle::default_bar().progress_chars("═█▓▒·"));
