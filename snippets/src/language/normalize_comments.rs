use crate::tree_sitter_consts::NODE_KIND_COMMENT;
use std::borrow::Cow;
use tree_sitter::Node;

/// Remove all comment node text from the given content.
/// In general, this function should work in any language that produces "comment" type nodes from tree_sitter.
#[tracing::instrument(skip_all)]
pub fn normalize_comments<'a, 'b>(
    context: &[Node<'_>],
    location: &crate::Location,
    content: &'a [u8],
) -> Cow<'a, [u8]> {
    let comment_nodes = context
        .into_iter()
        .filter(|n| n.kind() == NODE_KIND_COMMENT);
    let crate::ByteOffset(offset) = location.byte_offset;
    let mut start_byte = 0;
    let mut slices = Vec::new();

    // Find every non-comment section of text from the original content into a sequence
    for node in comment_nodes {
        let end_byte = node.start_byte() - offset;
        let next_start_byte = node.end_byte() - offset;
        slices.push(&content[start_byte..end_byte]);
        start_byte = next_start_byte;
    }

    slices.push(&content[start_byte..content.len()]);

    slices.concat().into()
}

mod tests {

    // #[test]
    // fn normalizes_comments() -> Cow<'_, [u8]> {
    //     //! Technically, `[normalize_comments]` has applications beyond C.
    //     //! For this test, only test with C.
    //     //! Language specific tests should also test this using code with comments and their respective syntax.
    //     let opts = SnippetOptions::new(SnippetTarget::full(),
    //                                    SnippetKinds::full(),
    //                                    SnippetTransforms::from(SnippetTransform::Comment)
    //     );

    //}
}
