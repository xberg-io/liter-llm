use serde::{Deserialize, Serialize};

/// Request to classify content for policy violations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModerationRequest {
    /// Text or texts to check.
    pub input: ModerationInput,
    /// Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Input to the moderation endpoint — a single string or multiple strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModerationInput {
    /// Single text string.
    Single(String),
    /// Multiple text strings (batch moderation).
    Multiple(Vec<String>),
}

#[cfg_attr(alef, alef(skip))]
impl Default for ModerationInput {
    fn default() -> Self {
        Self::Single(String::new())
    }
}

/// Response from the moderation endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationResponse {
    /// Unique identifier for this moderation request.
    pub id: String,
    /// Model used for classification.
    pub model: String,
    /// Results for each input string.
    pub results: Vec<ModerationResult>,
}

/// A single moderation classification result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationResult {
    /// True if any category was flagged.
    pub flagged: bool,
    /// Boolean flags for each moderation category.
    pub categories: ModerationCategories,
    /// Confidence scores for each category.
    pub category_scores: ModerationCategoryScores,
}

/// Boolean flags for each moderation category.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModerationCategories {
    /// Sexual content.
    #[serde(default)]
    pub sexual: bool,
    /// Hate speech.
    #[serde(default)]
    pub hate: bool,
    /// Harassment.
    #[serde(default)]
    pub harassment: bool,
    /// Self-harm content.
    #[serde(default, rename = "self-harm")]
    pub self_harm: bool,
    /// Sexual content involving minors.
    #[serde(default, rename = "sexual/minors")]
    pub sexual_minors: bool,
    /// Hate speech that threatens violence.
    #[serde(default, rename = "hate/threatening")]
    pub hate_threatening: bool,
    /// Graphic violence.
    #[serde(default, rename = "violence/graphic")]
    pub violence_graphic: bool,
    /// Intent to self-harm.
    #[serde(default, rename = "self-harm/intent")]
    pub self_harm_intent: bool,
    /// Instructions for self-harm.
    #[serde(default, rename = "self-harm/instructions")]
    pub self_harm_instructions: bool,
    /// Harassment that threatens violence.
    #[serde(default, rename = "harassment/threatening")]
    pub harassment_threatening: bool,
    /// Non-graphic violence.
    #[serde(default)]
    pub violence: bool,
}

/// Confidence scores for each moderation category.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModerationCategoryScores {
    /// Sexual content score.
    #[serde(default)]
    pub sexual: f64,
    /// Hate speech score.
    #[serde(default)]
    pub hate: f64,
    /// Harassment score.
    #[serde(default)]
    pub harassment: f64,
    /// Self-harm content score.
    #[serde(default, rename = "self-harm")]
    pub self_harm: f64,
    /// Sexual content involving minors score.
    #[serde(default, rename = "sexual/minors")]
    pub sexual_minors: f64,
    /// Hate speech that threatens violence score.
    #[serde(default, rename = "hate/threatening")]
    pub hate_threatening: f64,
    /// Graphic violence score.
    #[serde(default, rename = "violence/graphic")]
    pub violence_graphic: f64,
    /// Intent to self-harm score.
    #[serde(default, rename = "self-harm/intent")]
    pub self_harm_intent: f64,
    /// Instructions for self-harm score.
    #[serde(default, rename = "self-harm/instructions")]
    pub self_harm_instructions: f64,
    /// Harassment that threatens violence score.
    #[serde(default, rename = "harassment/threatening")]
    pub harassment_threatening: f64,
    /// Non-graphic violence score.
    #[serde(default)]
    pub violence: f64,
}
