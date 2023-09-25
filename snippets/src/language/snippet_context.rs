use crate::impl_prelude::SnippetLocation;
use getset::{CopyGetters, Getters};
use tracing::debug;
use tree_sitter::Node;

/// This structure represents a view into a larger piece of parsed text.
/// For snippet scanning, we generally look at just parts of a larger piece of text for each snippet.
/// However, the parsed nodes all reference locations in the original text.
/// This structure is meant to make it easier to find content inside a snippet based on previously extracted nodes.
#[derive(Debug, PartialEq, Getters, CopyGetters)]
pub struct SnippetContext<'a> {
    offset: usize,
    /// The location in the original text of this snippet.
    #[getset(get = "pub")]
    location: SnippetLocation,
    /// The nodes that have been parsed from this context.
    #[getset(get = "pub")]
    context_nodes: Vec<Node<'a>>,
    /// The slice of text represented by [`SnippetLocation`].
    #[getset(get_copy = "pub")]
    content: &'a [u8],
}

impl<'a> SnippetContext<'a> {
    /// Make a new SnippetContext from a sequence of nodes, a location within the original parsed text, and a sequence of bytes for data inside the snippet.
    /// There is no checking or guarantee that the provided nodes fall within the bounds of the provided content.
    pub fn new(context_nodes: Vec<Node<'a>>, location: SnippetLocation, content: &'a [u8]) -> Self {
        let crate::ByteOffset(offset) = location.byte_offset;
        SnippetContext {
            offset,
            context_nodes,
            location,
            content,
        }
    }

    pub fn filter_nodes(&self, p: impl FnMut(&Node<'a>) -> bool) -> SnippetContext {
        let new_nodes = self.context_nodes.iter().map(|i| i.clone());

        SnippetContext::new(new_nodes.filter(p).collect(), self.location.clone(), self.content)
    }

    /// Return all content described by nodes in this context.
    pub fn context_text(&self) -> Vec<u8> {
        let mut slices = Vec::new();
//        let mut last_start_byte = 0;
        let mut last_end_byte = 0;

        for n in self.context_nodes.iter() {
            let next_start_byte = n.start_byte();
            let end_byte = n.end_byte();

            if end_byte < last_end_byte {
                continue;
            }
            
            let start_byte = if next_start_byte < last_end_byte {
                last_end_byte
            } else {
                next_start_byte
            };

            print!("{} {}\n", start_byte, end_byte);
            print!("Node type: {}\n", n.kind());
            let slice = &self.content[start_byte .. end_byte - self.offset];
            print!("slice: {}\n", std::str::from_utf8(slice).expect("oops"));
            slices.push(slice);

            //last_start_byte = start_byte;
            last_end_byte = end_byte;
        }

        slices.concat()
    }
    
    /// Get content from the snippet which is not in ranges covered by the provided nodes.
    pub fn retrieve_content_around_nodes(
        &self,
        nodes: impl Iterator<Item = &'a Node<'a>>,
    ) -> impl Iterator<Item = &'a [u8]> {
        let mut start_byte = 0;
        let mut slices = Vec::new();

        for node in nodes {
            let end_byte = node.start_byte() - self.offset;
            let next_start_byte = node.end_byte() - self.offset;
            slices.push(&self.content[start_byte..end_byte]);
            start_byte = next_start_byte;
        }

        slices.push(&self.content[start_byte..self.content.len()]);

        slices.into_iter()
    }
}
