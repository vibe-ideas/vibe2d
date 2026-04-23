use serde::{Deserialize, Serialize};

/// Unique identifier for a UI widget.
///
/// Used for persistent state indexing (TextInput, ScrollList) and VDP widget targeting.
/// Stateful widgets require an explicit ID; stateless widgets auto-generate one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub String);

impl WidgetId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn auto(index: usize) -> Self {
        Self(format!("__auto_{}", index))
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
