use sprs::CsMat;
use std::{collections::HashMap, fs};

use crate::article;

struct AnalyzerData {
    keywords: Vec<String>,
    relations: CsMat<usize>,
    keyword_ratings: CsMat<f32>,
    n_keywords: usize,
}

impl AnalyzerData {
    pub fn new(n_keywords: usize, keywords: &Vec<String>) -> AnalyzerData {
        AnalyzerData {
            n_keywords,
            keywords: keywords.clone(),
            relations: CsMat::zero((n_keywords, n_keywords)),
            keyword_ratings: CsMat::zero((n_keywords, 10)),
        }
    }
}

pub struct Analyzer {
    filenames: Vec<String>,
    keyword_candidates: HashMap<String, usize>,
    lower_cutoff: f32,
    upper_cutoff: f32,
}

impl Analyzer {
    pub fn new(lower_cutoff: f32, upper_cutoff: f32) -> Self {
        Self {
            filenames: vec![],
            lower_cutoff,
            upper_cutoff,
            keyword_candidates: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        self.detect_input_files();
        self.analyze_dataset();
    }

    fn detect_input_files(&mut self) {
        let mut counter = 0;
        loop {
            let fname = format!("results_pubmed24n{:0>4}.xml.json", counter.clone());
            let file_exists = std::path::Path::new(&fname).exists();
            if file_exists {
                self.filenames.push(fname.clone());
                counter += 1;
            } else {
                break;
            }
        }
    }

    fn analyze_dataset(&mut self) -> AnalyzerData {
        for file in self.filenames.clone().iter() {
            self.analyze_one_input_file(file.clone());
        }

        self.purge_keyword_array();
        let keywords: Vec<String> = self
            .keyword_candidates
            .iter()
            .map(|k| k.0.clone())
            .collect();
        AnalyzerData::new(self.keyword_candidates.len(), &keywords)
    }

    fn analyze_one_input_file(&mut self, filename: String) {
        let file_contents: String = fs::read_to_string(filename).unwrap();
        let articles: Vec<article::Article> = serde_json::from_str(&file_contents).unwrap();
        for article in articles.iter() {
            self.process_abstract(article.paper_abstract.clone());
        }
    }

    fn process_abstract(&mut self, paper_abstract: String) {
        let mut cleared = paper_abstract.replace(".", " ");
        cleared = cleared.replace("?", " ");
        cleared = cleared.replace(";", " ");
        cleared = cleared.replace("(", " ");
        cleared = cleared.replace(")", " ");
        cleared = cleared.replace("!", " ");
        cleared = cleared.to_lowercase();
        for word in cleared.split_whitespace() {
            let counter = self.keyword_candidates.entry(word.to_string()).or_insert(0);
            *counter += 1;
        }
    }

    fn purge_keyword_array(&mut self) {
        let n_files = self.filenames.len() as f32;
        let lc = self.lower_cutoff * n_files;
        let uc = self.upper_cutoff * n_files;
        self.keyword_candidates
            .retain(|_, &mut count| (count as f32) > lc && (count as f32) < uc)
    }
}
