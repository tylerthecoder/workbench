use serde::{Deserialize, Serialize};

use crate::apps::{ToolKind, ToolState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bench {
    pub name: String,
    #[serde(default)]
    pub tool_defaults: Vec<ToolDefault>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolDefault {
    pub bay: u32,
    pub name: Option<String>,
    #[serde(default)]
    pub tool_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchRuntime {
    pub name: String,
    #[serde(default)]
    pub captured_bays: Vec<CapturedBay>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapturedBay {
    pub bay: u32,
    pub name: Option<String>,
    #[serde(default)]
    pub window_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceSnapshot {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub kind: ToolKind,
    #[serde(default)]
    pub state: Option<ToolState>,
}
