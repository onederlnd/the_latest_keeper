use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TutorialStep {
    pub id: String,
    pub trigger: String,
    pub text: String,
}
