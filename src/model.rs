use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::apps::{ToolKind, ToolState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bench {
    pub name: String,
    #[serde(default)]
    pub bays: Vec<BaySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaySpec {
    pub name: String,
    #[serde(default)]
    pub tool_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssembledBench {
    #[serde(default)]
    pub bay_windows: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssembledTool {
    pub window_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub kind: ToolKind,
    #[serde(default)]
    pub state: Option<ToolState>,
}
