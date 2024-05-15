use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Article {
    pub title: String,
    pub id: String,
    pub paper_abstract: String,
    pub authors: Vec<Author>,
    pub tags: Vec<String>,
    pub date: String,
    pub language: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Author {
    pub first_name: String,
    pub last_name: String,
}
