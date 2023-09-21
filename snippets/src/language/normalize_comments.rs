use crate::tree_sitter_consts::NODE_KIND_COMMENT;
use std::borrow::Cow;

use super::c99_tc3::SnippetContext;

/// Remove all comment node text from the given content.
/// In general, this function should work in any language that produces "comment" type nodes from tree_sitter.
#[tracing::instrument(skip_all)]
pub fn normalize_comments<'a>(
    context: &SnippetContext
) -> Cow<'a, [u8]> {
    let comment_nodes = context.context_nodes()
        .into_iter()
        .filter(|n| n.kind() == NODE_KIND_COMMENT);

    context.retrieve_negative_content(comment_nodes).collect::<Vec<_>>().concat().into()
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
