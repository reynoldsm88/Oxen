use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DataTypeStat {
    pub data_type: String,
    pub data_size: u64,
    pub file_count: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RepoStats {
    pub data_size: u64,
    pub data_types: HashMap<String, DataTypeStat>,
}
