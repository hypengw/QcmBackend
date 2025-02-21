use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
    pub id: String, 
    pub name: String,
    pub alias: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub picUrl: String,
}
