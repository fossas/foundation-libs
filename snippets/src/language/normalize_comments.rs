
use std::{borrow::Cow};
use tracing::debug;
use tree_sitter::Node;
use crate::tree_sitter_consts::NODE_KIND_COMMENT;

/// Remove all comment node text from the given content.
/// In general, this function should work in any language that produces "comment" type nodes from tree_sitter.
#[tracing::instrument(skip_all)]
pub fn normalize_comments<'a, 'b>(context: &[Node<'_>], location: &crate::Location, original_content: &'b [u8], content: &'a [u8]) -> Cow<'a, [u8]> {
    let comment_nodes = context
        .into_iter()
        .filter(|n| n.kind() == NODE_KIND_COMMENT);
    // debug!("CONTENT: {}", std::str::from_utf8(content).expect("expected to parse string"));
    debug!("CONTENT LENGTH: {}", content.len());
//    let crate::ByteOffset(loc_start) = location.byte_offset;
    let crate::ByteOffset(offset) = location.byte_offset;
    let mut start_byte = 0;
    let mut slices = Vec::new();
    
    for node in comment_nodes {
        let end_byte = node.start_byte() - offset;
        let next_start_byte = node.end_byte() - offset;
        let new_content = &content[start_byte..end_byte];
        debug!("PUSHING BYTES {:?} - {:?}\nWITH CONTENT: {:?}", start_byte, end_byte, std::str::from_utf8(new_content));
        debug!("FROM ORIGINAL NODE CONTENT: {:?}", node.utf8_text(original_content).expect("utf8 text"));
        debug!("FROM CONTEXT NODE CONTENT: {:?}", std::str::from_utf8(&content[node.start_byte() - offset..node.end_byte() - offset]));
        slices.push(new_content);
        start_byte = next_start_byte;
    }
    slices.push(&content[start_byte..content.len()]);

    let all_slices: Cow<'a, [u8]> = slices.concat().into();
    debug!("ALL SLICES: {:?}", std::str::from_utf8(&all_slices).expect("utf8 text"));
    all_slices
}

mod tests{
        
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
