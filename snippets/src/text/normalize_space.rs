use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::bytes::Regex;

static MULTI_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").expect("regex must be valid"));

/// Normalize any consecutive whitespace into a single space.
#[tracing::instrument(skip_all)]
pub fn normalize_space(text: &[u8]) -> Cow<'_, [u8]> {
    MULTI_SPACES.replace_all(text.as_ref(), b" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_spaces_compiles() {
        let _ = MULTI_SPACES;
    }

    #[test]
    fn normalizes_spaces() {
        let input = b"Sit  dolorem\nconsequatur tenetur \n porro\n\naspernatur\t\n .";
        //               ^^       ^            ^       ^^ ^     ^ ^           ^ ^ ^

        let expected = b"Sit dolorem consequatur tenetur porro aspernatur .";
        assert_eq!(normalize_space(input).as_ref(), expected);
    }
}
