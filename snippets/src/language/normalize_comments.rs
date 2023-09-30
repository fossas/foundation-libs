use crate::tree_sitter_consts::NODE_KIND_COMMENT;

use super::snippet_context::SnippetContext;

/// Remove all comment node text from the given content.
/// In general, this function should work in any language for which treesitter produces nodes whose `kind` equals [`NODE_KIND_COMMENT`].
///
/// Because tree sitter parsers can define their own node types
/// it's possible a comment node may have a kind not equal to [`NODE_KIND_COMMENT`].
/// Extractors that use this function should test each language-specific comment syntax at least once.
#[tracing::instrument(skip_all)]
pub fn normalize_comments<'a>(context: &'a SnippetContext) -> Vec<u8> {
    let comment_nodes = context
        .nodes()
        .iter()
        .filter(|n| n.kind() == NODE_KIND_COMMENT);
    context.content_around(comment_nodes)
}

#[cfg(test)]
mod tests {

    use crate::impl_prelude::SnippetLocation;
    use crate::language::snippet_context::SnippetContext;
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
        let context = SnippetContext::from_nodes(
            traverse_tree(&tree, Order::Pre),
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
