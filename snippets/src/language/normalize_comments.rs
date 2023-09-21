use crate::tree_sitter_consts::NODE_KIND_COMMENT;
use std::borrow::Cow;

use super::context::SnippetContext;

/// Remove all comment node text from the given content.
/// In general, this function should work in any language that produces "comment" type nodes from tree_sitter.
#[tracing::instrument(skip_all)]
pub fn normalize_comments<'a>(context: &'a SnippetContext) -> Cow<'a, [u8]> {
    let comment_nodes = context
        .context_nodes()
        .into_iter()
        .filter(|n| n.kind() == NODE_KIND_COMMENT);

    context
        .retrieve_negative_content(comment_nodes)
        .collect::<Vec<&'a [u8]>>()
        .concat()
        .into()
}

#[cfg(test)]
mod tests {

    use crate::impl_prelude::SnippetLocation;
    use crate::language::context::SnippetContext;
    use tree_sitter_traversal::{traverse_tree, Order};

    #[cfg(feature = "lang-c99-tc3")]
    #[test]
    fn normalizes_comments() {
        //! Technically, `[normalize_comments]` has applications beyond C.
        //! This is meant to be a very basic test and uses C.
        //! Language specific tests should also be done against this during full fingerprinting.
        let text = r#"int main() {
  printf("Hello, world!"); // comment
  /* A longer comment */
}"#
        .as_bytes();
        let expected_text = r#"int main() {
  printf("Hello, world!"); 
  
}"#;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_c::language())
            .expect("Could not set language");
        let tree = parser.parse(text, None).expect("Couldn't parse test text");
        let nodes = traverse_tree(&tree, Order::Pre).collect();

        let context = SnippetContext::new(
            nodes,
            SnippetLocation::builder()
                .byte_offset(0)
                .byte_len(text.len())
                .build(),
            text,
        );
        let out_text = super::normalize_comments(&context);
        assert_eq!(
            std::str::from_utf8(out_text.as_ref()).expect("Could not parse out text"),
            expected_text
        );
    }
}
