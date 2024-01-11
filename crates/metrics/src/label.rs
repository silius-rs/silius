/// Represents a label-value pair.
/// Mainly used for metrics system.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelValue {
    pub label: String,
    pub value: String,
}

impl LabelValue {
    pub fn new(label: String, value: String) -> Self {
        Self { label, value }
    }
}
