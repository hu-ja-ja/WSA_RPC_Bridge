// ponytail: wash before search — drop trailing tails that pollute matching
pub(crate) fn wash_title(raw: &str) -> String {
    let mut cur = raw.trim().to_string();
    loop {
        let next = strip_one_trailing_group(&cur);
        if next.len() == cur.len() {
            break;
        }
        cur = next;
        if cur.trim().is_empty() {
            return String::new();
        }
    }
    cur.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_one_trailing_group(s: &str) -> String {
    const PAIRS: [(char, char); 6] = [
        ('(', ')'),
        ('[', ']'),
        ('\u{3010}', '\u{3011}'),
        ('\u{300c}', '\u{300d}'),
        ('\u{300e}', '\u{300f}'),
        ('\u{ff08}', '\u{ff09}'),
    ];
    let trimmed = s.trim_end();
    let close = match trimmed.chars().last() {
        Some(c) => c,
        None => return s.to_string(),
    };
    let open = match PAIRS.iter().find(|(_, cl)| *cl == close) {
        Some((op, _)) => *op,
        None => return s.to_string(),
    };
    let open_at = match trimmed.rfind(open) {
        Some(i) => i,
        None => return s.to_string(),
    };
    let inner = &trimmed[open_at + open.len_utf8()..trimmed.len() - close.len_utf8()];
    if !is_washable_tail(inner) {
        return s.to_string();
    }
    trimmed[..open_at].trim_end().to_string()
}

fn is_washable_tail(inner: &str) -> bool {
    const LONG: [&str; 8] = [
        "official", "video", "audio", "visual", "lyric", "remaster", "mono", "stereo",
    ];
    const SHORT: [&str; 5] = ["mv", "hq", "hd", "4k", "8k"];
    let low = inner.to_lowercase();
    if LONG.iter().any(|m| low.contains(m)) {
        return true;
    }
    low.split(|c: char| !c.is_alphanumeric())
        .any(|tok| SHORT.contains(&tok))
}

// ponytail: channel tail + second and later names cut for search words only
pub(crate) fn wash_artist(raw: &str) -> String {
    let mut cur = raw.trim().to_string();
    if cur.to_lowercase().ends_with("- topic") {
        let cut = cur.len() - "- topic".len();
        cur = cur[..cut]
            .trim_end()
            .trim_end_matches('-')
            .trim_end()
            .to_string();
    }
    let low = cur.to_lowercase();
    let mut cut_at: Option<usize> = None;
    const HARD: [&str; 6] = [" & ", ",", "\u{3001}", "\u{ff0c}", ";", "\u{ff1b}"];
    for sep in HARD {
        if let Some(i) = cur.find(sep) {
            if i > 0 && cut_at.map_or(true, |c| i < c) {
                cut_at = Some(i);
            }
        }
    }
    const SOFT: [&str; 7] = [
        " feat",
        " ft",
        " with ",
        " vs ",
        " plus ",
        " x ",
        " \u{00d7} ",
    ];
    for sep in SOFT {
        if let Some(i) = low.find(sep) {
            if i > 0 && cut_at.map_or(true, |c| i < c) {
                cut_at = Some(i);
            }
        }
    }
    if let Some(i) = cut_at {
        cur = cur[..i].to_string();
    }
    cur.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ponytail: lowercase + normalize brackets/separators to space, keep inner text (version distinction like feat)
pub(crate) fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    for c in s.chars() {
        match c {
            '(' | '[' | '\u{3010}' | '\u{FF08}' | '\u{300E}' | '\u{300C}' | ')' | ']'
            | '\u{3011}' | '\u{FF09}' | '\u{300F}' | '\u{300D}' | '"' | '\'' | '`' | '“' | '”'
            | '‘' | '’' => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
                continue;
            }
            // separators like dash/colon that often split title/artist: normalize to space
            '-' | '–' | '—' | ':' | '：' | '・' | '.' | '·' | '_' | '/' | '／' => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c if c.is_whitespace() => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c => {
                out.extend(c.to_lowercase());
                prev_space = false;
            }
        }
    }
    out.trim_end().to_string()
}

// ponytail: hypothesis list — full string first, then comma-split parts; no need to decide which split is right
pub(crate) fn artist_candidates(artist: &str) -> Vec<String> {
    let mut out = Vec::new();
    let full = normalize(artist);
    if !full.is_empty() {
        out.push(full.clone());
        let nospace = full.replace(' ', "").replace('　', "");
        if !nospace.is_empty() && !out.contains(&nospace) {
            out.push(nospace);
        }
    }
    for part in artist.split([',', '\u{3001}', '\u{FF0C}']) {
        let n = normalize(part);
        if !n.is_empty() && !out.contains(&n) {
            out.push(n.clone());
            let ns = n.replace(' ', "").replace('　', "");
            if !ns.is_empty() && !out.contains(&ns) {
                out.push(ns);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_brackets_and_spaces() {
        assert_eq!(
            normalize("Test Song (Official Video)"),
            "test song official video"
        );
        assert_eq!(normalize("タイトル【特典付き】"), "タイトル 特典付き");
        assert_eq!(normalize("  A   B  "), "a b");
        assert_eq!(normalize("（全角）"), "全角");
    }

    #[test]
    fn wash_title_strips_tails_only() {
        assert_eq!(wash_title("Song Name (Official Video)"), "Song Name");
        assert_eq!(wash_title("Song Name [Remastered]"), "Song Name");
        assert_eq!(wash_title("Song Name (HD)"), "Song Name");
        assert_eq!(wash_title("Song Name (Live)"), "Song Name (Live)");
        assert_eq!(wash_title("Song Name"), "Song Name");
    }

    #[test]
    fn wash_artist_keeps_first_only() {
        assert_eq!(wash_artist("Someone - Topic"), "Someone");
        assert_eq!(wash_artist("A, B"), "A");
        assert_eq!(wash_artist("A & B"), "A");
        assert_eq!(wash_artist("Solo"), "Solo");
    }
}
