//! Internal request/response types.
//!
//! These are the *neutral* shape that the router and ledger see. The OpenAI
//! request parser and every provider adapter convert to/from these types.
//! Keeping a single internal shape lets us add providers without leaking
//! their quirks into the router.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    /// Free-form text content. For tool-call / tool-result messages we
    /// stash the structured payload in `extra` rather than inventing a
    /// union here. `extra` is provider-adapter escape space, not part of
    /// the routing surface; rule predicates should use normalized fields.
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    /// JSON schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

/// Internal, provider-neutral request. The OpenAI parser populates this
/// from the wire format; the router and cascade operate on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// The model alias the client *asked for*. May be `"auto"` or any alias
    /// declared in config — the router has the final say.
    pub requested_model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

impl Request {
    /// Rough token estimate over the prompt. Used by rule predicates.
    /// We don't need exactness here — order of magnitude is enough for
    /// routing decisions.
    pub fn estimate_input_tokens(&self) -> usize {
        // Conservative approximation: agent prompts often contain code/JSON,
        // where 4 chars/token undercounts. Err slightly high for routing.
        let chars: usize = self.messages.iter().map(|m| m.content.len()).sum();
        (chars / 3) + (self.messages.len() * 4)
    }

    pub fn system_text(&self) -> String {
        self.messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn has_tools(&self) -> bool {
        !self.tools.is_empty()
    }
}

/// A single response from a provider. For streaming responses, the
/// adapter emits a stream of `ResponseChunk`s and we assemble usage at the
/// end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub model: String,
    pub content: String,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    /// Echoed back to the caller in the OpenAI shape; carried through
    /// unmodified.
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
}

/// A single SSE-style chunk in a streaming response.
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub delta_text: String,
    pub done: bool,
    /// Set on the final chunk.
    pub finish_reason: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}
