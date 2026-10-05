use crate::{MAX_COMPONENTS, MAX_REFERENCE_BYTES, MAX_REFERENCE_UTF16, UncheckedReason};

/// Syntax only; this is neither a locator nor filesystem authority. No Debug.
pub struct AbsolutePath {
    drive: u8,
    components: Vec<String>,
}

impl AbsolutePath {
    pub fn parse(value: &str) -> Result<Self, UncheckedReason> {
        use UncheckedReason::*;
        if value.len() > MAX_REFERENCE_BYTES || value.encode_utf16().count() > MAX_REFERENCE_UTF16 {
            return Err(LimitReached);
        }
        if value.is_empty() || value.chars().any(unsafe_control) {
            return Err(UnsupportedPathSyntax);
        }
        if value.contains('%') || value.starts_with('$') {
            return Err(UnresolvedPlaceholder);
        }
        if value.starts_with(['\\', '/']) || value.contains("://") {
            return Err(UnsupportedPathSyntax);
        }
        let bytes = value.as_bytes();
        if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
            return Err(if value.contains(':') {
                UnsupportedPathSyntax
            } else {
                RelativeReference
            });
        }
        if bytes.get(2) != Some(&b'\\') {
            return Err(UnsupportedPathSyntax);
        }
        // Conservative Win32 spelling: no alternate separator, normalization,
        // stream, short-name alias or trailing separator. The drive root itself
        // is allowed so an exact root target can be reported as a directory.
        let tail = &value[3..];
        let components: Vec<String> = if tail.is_empty() {
            Vec::new()
        } else {
            tail.split('\\').map(str::to_owned).collect()
        };
        if components.len() > MAX_COMPONENTS {
            return Err(LimitReached);
        }
        for component in &components {
            if component.is_empty()
                || component == "."
                || component == ".."
                || component.ends_with(['.', ' '])
                || component.contains(['/', ':', '*', '?', '"', '<', '>', '|', '~'])
                || reserved(component)
            {
                return Err(UnsupportedPathSyntax);
            }
        }
        Ok(Self {
            drive: bytes[0].to_ascii_uppercase(),
            components,
        })
    }

    pub fn drive(&self) -> u8 {
        self.drive
    }
    pub fn components(&self) -> &[String] {
        &self.components
    }
}

fn unsafe_control(value: char) -> bool {
    value.is_control()
        || matches!(value, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}')
}

fn reserved(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        upper.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}
