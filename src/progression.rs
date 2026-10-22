use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Tier {
    pub discovery: f32,
    pub milestone: f32,
}
