use std::io::{self, BufRead, BufReader, Cursor, Read, Write};

use iter_read::IterRead;
use sha2::{Digest, Sha256};

use crate::{stream::ConvertCRLFToLF, CommentStrippedSHA256, Error, Fingerprint, RawSHA256};

/// Fingerprint the file using the [`RawSHA256`] kind.
pub fn raw<R: Read>(stream: &mut R) -> Result<Fingerprint<RawSHA256>, Error> {
    // Read the start of the stream, and decide whether to treat the rest of the stream as binary based on that.
    let BinaryCheck { read, is_binary } = content_is_binary(stream)?;

    // Chain the part of the stream already read to evaluate binary along with the rest of the stream.
    let mut stream = Cursor::new(read).chain(stream);
    let print = if is_binary {
        hash_binary(&mut stream)
    } else {
        hash_text(&mut stream)
    }?;
    Ok(print)
}

/// Fingerprint the file using the [`CommentStrippedSHA256`] kind.
pub fn comment_stripped<R: Read>(
    stream: &mut R,
) -> Result<Option<Fingerprint<CommentStrippedSHA256>>, Error> {
    // Read the start of the stream, and decide whether to treat the rest of the stream as binary based on that.
    let BinaryCheck { read, is_binary } = content_is_binary(stream)?;
    if is_binary {
        return Ok(None);
    }

    // Chain the part of the stream already read to evaluate binary along with the rest of the stream.
    let mut stream = Cursor::new(read).chain(stream);
    match hash_text_stripped(&mut stream) {
        Ok(fp) => Ok(Some(fp)),
        Err(err) => {
            // The `io::Error` type is opaque.
            // Handle the case of attempting to comment strip a binary file.
            if err.to_string().to_lowercase().contains("utf-8") {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}

struct BinaryCheck {
    read: Vec<u8>,
    is_binary: bool,
}

/// Inspect the file to determine if it is binary.
///
/// Uses the same method as git: "is there a zero byte in the first 8000 bytes of the file"
fn content_is_binary<R: Read>(stream: &mut R) -> Result<BinaryCheck, io::Error> {
    let mut buf = Vec::new();
    stream.take(8000).read_to_end(&mut buf)?;
    let is_binary = buf.contains(&0);
    Ok(BinaryCheck {
        read: buf,
        is_binary,
    })
}

/// Hashes the exact contents of a binary file without modification.
fn hash_binary<R: Read>(stream: &mut R) -> Result<Fingerprint<RawSHA256>, Error> {
    let mut hasher = Sha256::new();
    io::copy(stream, &mut hasher)?;
    Fingerprint::from_digest(hasher)
}

/// Hashes text files in a platform independent manner.
///
/// Specifically:
/// - All text encodings are ignored; this function operates on raw bytes.
/// - `git` implementations on Windows typically check out files with `\r\n` line endings,
///   while *nix checks them out with `\n`.
///   To be platform independent, any `\r\n` byte sequences found are converted to a single `\n`.
fn hash_text<R: Read>(stream: &mut R) -> Result<Fingerprint<RawSHA256>, Error> {
    let stream = BufReader::new(stream).bytes().crlf_to_lf().fuse();
    let mut hasher = Sha256::new();
    io::copy(&mut IterRead::new(stream), &mut hasher)?;
    Fingerprint::from_digest(hasher)
}

/// Hashes code files while removing C-style comments and blank lines in a platform independent manner.
///
/// Specifically:
/// - All text encodings are treated as utf8.
/// - `git` implementations on Windows typically check out files with `\r\n` line endings,
///   while *nix checks them out with `\n`.
///   To be platform independent, any `\r\n` byte sequences found are converted to a single `\n`.
/// - C-style comments are removed:
///   - `//` is considered the start of a single line comment; these bytes and any other bytes until right before a `\n` are removed.
///   - `/*` is considered the start of a multi line comment; these bytes and any other bytes until after a `*/` is read are removed.
///   - This function does not check for escaped comments.
/// - Any sequence of multiple contiguous `\n` bytes are collapsed to a single `\n` byte.
/// - The final `\n` byte is removed from the end of the stream if present.
fn hash_text_stripped<R: Read>(
    stream: &mut R,
) -> Result<Fingerprint<CommentStrippedSHA256>, Error> {
    let mut hasher = Sha256::new();
    comment_strip(stream, &mut hasher)?;
    Fingerprint::from_digest(hasher)
}

fn comment_strip<R: Read, W: Write>(stream: &mut R, w: &mut W) -> Result<(), Error> {
    let mut buffered_output_line = String::new();
    let mut is_multiline_active = false;

    for line in BufReader::new(stream).lines() {
        let line = line?;

        // At this point we know we have a new line coming. If a previous line is buffered and ready to write, do so now.
        // Write it with a trailing newline because we know we'll be writing a following line.
        if !buffered_output_line.is_empty() {
            writeln!(w, "{buffered_output_line}")?;
        }

        (buffered_output_line, is_multiline_active) = clean_line(line, is_multiline_active);
        buffered_output_line = buffered_output_line.trim().to_owned();
    }

    // Now that we're done reading the input stream, if there's a buffered output line write it *without a trailing newline*.
    write!(w, "{buffered_output_line}")?;
    Ok(())
}

/// Part comment stripping, part state machine. Cleans lines of comments based on whether a previous invocation
/// detected the start of a multi line comment.
///
/// This is very much not an ideal function: it scans the line multiple times instead of being forward-looking-only,
/// and the dual responsibility makes it complicated. We should fix this, but moving forward for now.
fn clean_line(line: String, is_multiline_active: bool) -> (String, bool) {
    if is_multiline_active {
        if let Some(end) = line.find("*/") {
            return clean_line(line[end + 2..].to_string(), false);
        }

        (String::new(), true)
    } else if let Some(start) = line.find("/*") {
        let before_multi = line[..start].to_string();
        let (after_multi, is_multi) = clean_line(line[start + 2..].to_string(), true);
        (before_multi + &after_multi, is_multi)
    } else if let Some(start) = line.find("//") {
        (line[..start].to_string(), false)
    } else {
        (line, false)
    }
}

#[cfg(test)]
mod tests {
    //! Tests for internal logic.

    use super::*;

    /// Inspired by the Haskell implementation: https://github.com/fossas/fossa-cli/blob/8de74b71b80d77321d64f94d7573773e49306772/test/App/Fossa/VSI/testdata/multi_line_comment.c#L1-L10
    #[test]
    fn comment_strip_mixed() {
        let content = r#"/*
 * This is a placeholder file used to test comment stripping code.
*/
    
int main() {
  int code = 0;
  // code = 1;




  return code; // perfect
}
"#;
        let expected = r#"int main() {
int code = 0;
return code;
}"#;

        let mut buf = Vec::new();
        comment_strip(&mut Cursor::new(content), &mut buf).expect("must fingerprint");
        assert_eq!(expected, String::from_utf8_lossy(&buf));
    }

    /// Copied from the Go implementation: https://github.com/fossas/basis/blob/6b0a1ce7ca5d88d033732f6dcfebd90b8f143038/sherlock/pkg/lib/indexer/cleaned/strip_comments_internal_test.go#L71-L79
    #[test]
    fn comment_strip_single_line_comments() {
        let content = " content1 \n content2 //comment \n content3 ";
        let expected = "content1\ncontent2\ncontent3";

        let mut buf = Vec::new();
        comment_strip(&mut Cursor::new(content), &mut buf).expect("must fingerprint");
        assert_eq!(expected, String::from_utf8_lossy(&buf));
    }

    /// Copied from the Go implementation: https://github.com/fossas/basis/blob/6b0a1ce7ca5d88d033732f6dcfebd90b8f143038/sherlock/pkg/lib/indexer/cleaned/strip_comments_internal_test.go#L89-L97
    #[test]
    fn comment_strip_multi_line_comments() {
        let content =
            " content1 \n  content2 /* begin comment \n end comment */ content3 \n content4 ";
        let expected = "content1\ncontent2\ncontent3\ncontent4";

        let mut buf = Vec::new();
        comment_strip(&mut Cursor::new(content), &mut buf).expect("must fingerprint");
        assert_eq!(expected, String::from_utf8_lossy(&buf));
    }

    #[test]
    fn comment_strip_cr() {
        let content = "hello world\r\nanother line\r\na final line\n";
        let expected = "hello world\nanother line\na final line";

        let mut buf = Vec::new();
        comment_strip(&mut Cursor::new(content), &mut buf).expect("must fingerprint");
        assert_eq!(expected, String::from_utf8_lossy(&buf));
    }
}
