use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Article {
    pub title: String,
    pub pmid: String,
    pub doi: String,
    pub pmc: String,
    pub pii: String,
    pub paper_abstract: String,
}
