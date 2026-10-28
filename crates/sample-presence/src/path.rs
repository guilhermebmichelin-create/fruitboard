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

/// A relative reference after Fruitboard's bounded, in-memory interpretation.
/// It contains the full candidate stack, including the selected-root prefix.
/// This is syntax only; its caller must first qualify the captured source and
/// root before treating the source's parent as an authorized base.
pub(crate) struct RelativeCandidate {
    drive: u8,
    components: Vec<String>,
}

impl RelativeCandidate {
    /// Interpret a relative saved reference from the captured source's parent.
    /// Call only after `MetadataPort::qualify` has established the root/source
    /// and source-parent/name authority represented by these captured paths.
    pub(crate) fn parse(
        value: &str,
        root: &AbsolutePath,
        source: &AbsolutePath,
    ) -> Result<Self, UncheckedReason> {
        use UncheckedReason::*;

        if value.len() > MAX_REFERENCE_BYTES || value.encode_utf16().count() > MAX_REFERENCE_UTF16 {
            return Err(LimitReached);
        }
        if value.is_empty()
            || value
                .chars()
                .any(|character| unsafe_control(character) || character == '\u{061c}')
        {
            return Err(UnsupportedPathSyntax);
        }
        if value.contains('%') {
            return Err(UnresolvedPlaceholder);
        }
        if value.starts_with(['\\', '/']) || value.contains("://") || value.contains(':') {
            return Err(UnsupportedPathSyntax);
        }
        if root.drive() != source.drive() || source.components().len() <= root.components().len() {
            return Err(OutsideRoot);
        }

        let raw: Vec<&str> = value.split('\\').collect();
        if raw.iter().any(|component| component.starts_with('$')) {
            return Err(UnresolvedPlaceholder);
        }
        if raw.len() > MAX_COMPONENTS {
            return Err(LimitReached);
        }

        // Validate the complete spelling before applying any dot-token
        // cancellation. A cancelled name still has to be safe as text.
        for component in &raw {
            if component.is_empty() {
                return Err(UnsupportedPathSyntax);
            }
            if *component == "." || *component == ".." {
                continue;
            }
            if component.ends_with(['.', ' '])
                || component.contains(['/', ':', '*', '?', '"', '<', '>', '|', '~'])
                || reserved(component)
            {
                return Err(UnsupportedPathSyntax);
            }
        }

        // Qualification is the trust boundary: this parent is taken from the
        // authorized captured source only after its held namespace binding is
        // established. Do not substitute the scan root or a process directory.
        let source_components = source.components();
        let mut components = source_components[..source_components.len() - 1].to_vec();
        let root_depth = root.components().len();
        if components.len() < root_depth {
            return Err(OutsideRoot);
        }
        if components.len() > MAX_COMPONENTS {
            return Err(LimitReached);
        }

        for component in raw {
            match component {
                "." => {}
                ".." => {
                    if components.len() <= root_depth {
                        return Err(OutsideRoot);
                    }
                    components.pop();
                }
                ordinary => {
                    if components.len() >= MAX_COMPONENTS {
                        return Err(LimitReached);
                    }
                    components.push(ordinary.to_owned());
                }
            }
        }

        Ok(Self {
            drive: root.drive(),
            components,
        })
    }

    pub(crate) fn drive(&self) -> u8 {
        self.drive
    }

    pub(crate) fn components(&self) -> &[String] {
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
