use crate::impl_prelude::SnippetLocation;
use getset::{CopyGetters, Getters};
use tree_sitter::Node;
use tree_sitter_traversal::{traverse, Order};

/// This structure represents a view into a larger piece of parsed text.
/// For snippet scanning, we generally look at just parts of a larger piece of text for each snippet.
#[derive(Debug, PartialEq, Getters, CopyGetters)]
pub struct SnippetContext<'a> {
    /// The location (in `content`) of this snippet.
    #[getset(get_copy = "pub")]
    location: SnippetLocation,

    /// Parsed nodes representing the snippet.
    #[getset(get = "pub")]
    nodes: Vec<Node<'a>>,

    /// The full text in which this snippet resides.
    content: &'a [u8],
}

impl<'a> SnippetContext<'a> {
    /// Make a new instance from a parent node and its location within the original parsed text.
    ///
    /// Ensure that the content provided is the same as the content used to extract the parent node;
    /// byte offsets must line up for operations on this type to make sense.
    pub fn new(parent: Node<'a>, location: SnippetLocation, content: &'a [u8]) -> Self {
        Self::from_nodes(traverse(parent.walk(), Order::Pre), location, content)
    }

    /// Make a new instance from a set of nodes and their location within the original parsed text.
    ///
    /// Ensure that the content provided is the same as the content used to extract the nodes;
    /// byte offsets must line up for operations on this type to make sense.
    pub fn from_nodes(
        nodes: impl IntoIterator<Item = Node<'a>>,
        location: SnippetLocation,
        content: &'a [u8],
    ) -> Self {
        SnippetContext {
            nodes: nodes.into_iter().collect(),
            location,
            content,
        }
    }

    /// Extract the part of the content indicated by the location.
    pub fn content(&self) -> &[u8] {
        self.location.extract_from(self.content)
    }

    /// Get content from the snippet which is not in ranges covered by the provided nodes.
    pub fn content_around(&self, nodes: impl Iterator<Item = &'a Node<'a>>) -> Vec<u8> {
        let mut slices = Vec::new();
        let mut start_byte = self.location.start_byte();

        for node in nodes {
            let node_start_byte = node.start_byte();
            slices.push(&self.content[start_byte..node_start_byte]);
            start_byte = node.end_byte();
        }

        // `Location::end_byte` is inclusive.
        slices.push(&self.content[start_byte..=self.location.end_byte()]);
        slices.concat()
    }
}
