// src/cellars.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellarRecord {
    pub name: String,
    pub years: String,
    pub epitaph: String,
    pub sacrificed_to: String,
    pub village: String,
}
