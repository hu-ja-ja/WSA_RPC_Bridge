use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub package_name: String,
    pub thumbnail_url: Option<String>,
    pub position: Option<u64>,
    pub duration: Option<u64>,
    pub display_name: Option<String>,
    pub is_playing: bool,
}

// ponytail: log masking, head only + length
pub fn mask_text(s: &str) -> String {
    let len = s.chars().count();
    if len == 0 {
        return String::from("(empty)");
    }
    let head: String = s.chars().take(2).collect();
    format!("{head}...({len})")
}

// ponytail: file=masked / stdout=raw(dev only). filters in log_plugin
pub const MASKED_TARGET: &str = "wsa_masked";
pub const RAW_TARGET: &str = "wsa_raw";

#[macro_export]
macro_rules! raw_info {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            log::info!(target: $crate::models::RAW_TARGET, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! raw_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            log::debug!(target: $crate::models::RAW_TARGET, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! raw_warn {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            log::warn!(target: $crate::models::RAW_TARGET, $($arg)*);
        }
    };
}

// ponytail: masked+raw pair in one call. wrap PII in m(), rest passes to both.
// e.g. mlog!(info, "title={}", m(&info.title)) — file gets masked, stdout gets raw.
#[macro_export]
macro_rules! mlog {
    ($lvl:ident, $fmt:expr $(, $($args:tt)+)?) => {
        $crate::mlog!(@collect $lvl, $fmt, masked[], raw[] $(, $($args)+)?)
    };
    (@collect $lvl:ident, $fmt:expr, masked[], raw[]) => {{
        log::$lvl!(target: $crate::models::MASKED_TARGET, $fmt);
        if cfg!(debug_assertions) {
            log::$lvl!(target: $crate::models::RAW_TARGET, $fmt);
        }
    }};
    (@collect $lvl:ident, $fmt:expr, masked[$($mk:expr),+], raw[$($rw:expr),+]) => {{
        log::$lvl!(target: $crate::models::MASKED_TARGET, $fmt, $($mk),+);
        if cfg!(debug_assertions) {
            log::$lvl!(target: $crate::models::RAW_TARGET, $fmt, $($rw),+);
        }
    }};
    (@collect $lvl:ident, $fmt:expr, masked[$($mk:expr),*], raw[$($rw:expr),*], m($e:expr), $($rest:tt)+) => {
        $crate::mlog!(@collect $lvl, $fmt,
            masked[$($mk,)* $crate::models::mask_text($e)],
            raw[$($rw,)* $e], $($rest)+)
    };
    (@collect $lvl:ident, $fmt:expr, masked[$($mk:expr),*], raw[$($rw:expr),*], m($e:expr) $(,)?) => {
        $crate::mlog!(@collect $lvl, $fmt,
            masked[$($mk,)* $crate::models::mask_text($e)],
            raw[$($rw,)* $e])
    };
    (@collect $lvl:ident, $fmt:expr, masked[$($mk:expr),*], raw[$($rw:expr),*], $head:expr, $($rest:tt)+) => {
        $crate::mlog!(@collect $lvl, $fmt,
            masked[$($mk,)* $head], raw[$($rw,)* $head], $($rest)+)
    };
    (@collect $lvl:ident, $fmt:expr, masked[$($mk:expr),*], raw[$($rw:expr),*], $head:expr $(,)?) => {
        $crate::mlog!(@collect $lvl, $fmt,
            masked[$($mk,)* $head], raw[$($rw,)* $head])
    };
}

// ponytail: verbose mlog — info in debug, debug in release. kills cfg! branches.
#[macro_export]
macro_rules! vlog {
    ($fmt:expr $(, $($args:tt)+)?) => {{
        if cfg!(debug_assertions) {
            $crate::mlog!(info, $fmt $(, $($args)+)?);
        } else {
            $crate::mlog!(debug, $fmt $(, $($args)+)?);
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_keeps_head_only() {
        assert_eq!(mask_text(""), "(empty)");
        assert_eq!(mask_text("あ"), "あ...(1)");
        assert_eq!(mask_text("test song"), "te...(9)");
    }

    // ponytail: macro expansion smoke test (no logger init — log! is a no-op)
    #[test]
    fn mlog_vlog_expand() {
        let title = String::from("song");
        let n = 3;
        crate::mlog!(info, "t={} n={}", m(&title), n);
        crate::mlog!(debug, "plain");
        crate::vlog!("v={}", m(&title));
    }
}
