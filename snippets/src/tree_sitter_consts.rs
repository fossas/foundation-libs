
pub const NODE_KIND_PARAM_LIST: &str = "parameter_list";
pub const NODE_KIND_COMMENT: &str = "comment";
pub const NODE_KIND_FUNC_DEF: &str = "function_definition";

// /// Enum for all the node kinds used by tree_sitter that we care to look at.
// /// Use [`From::from`] or [`Into::into`] to get a NodeKind from a string.
// #[derive(PartialEq)]
// pub enum NodeKind {
//     ParamList,
//     Comment,
//     FunctionDefinition,
//     /// If you find yourself getting this NodeKind, it's a strong indication that you should extend this enum.
//     Unknown
// }

// impl From<&str> for NodeKind {
//     fn from(s: &str) -> NodeKind {
//         match s {
//             NODE_KIND_PARAM_LIST => NodeKind::ParamList,
//             NODE_KIND_COMMENT => NodeKind::Comment,
//             NODE_KIND_FUNC_DEF => NodeKind::FunctionDefinition,
//             _ => NodeKind::Unknown
//         }
//     }
// }
