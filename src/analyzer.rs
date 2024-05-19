use crate::analyzer_data::AnalyzerData;
use crate::article;
use std::collections::HashMap;
use std::fs;

pub struct Analyzer {
    filenames: Vec<String>,
    keyword_candidates: HashMap<String, usize>,
    lower_cutoff: f32,
    upper_cutoff: f32,
    bar_style: indicatif::ProgressStyle,
}

impl Analyzer {
    pub fn new(lower_cutoff: f32, upper_cutoff: f32) -> Self {
        let bar_style = indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>5}/{len:5} {eta}",
        )
        .unwrap()
        .progress_chars("##-");
        Self {
            filenames: vec![],
            lower_cutoff,
            upper_cutoff,
            keyword_candidates: HashMap::new(),
            bar_style,
        }
    }

    pub fn run(&mut self) {
        self.detect_input_files();
        let mut analyzer_data = self.analyze_dataset();
        self.build_relations_matrix(&mut analyzer_data);
        analyzer_data.print();
        analyzer_data.compute_keyword_ratings();
        analyzer_data.write_rating_output();
    }

    fn build_relations_matrix(&self, analyzer: &mut AnalyzerData) {
        let bar = indicatif::ProgressBar::new(self.filenames.len() as u64);
        bar.set_style(self.bar_style.clone());
        for file in self.filenames.iter() {
            let file_contents: String = fs::read_to_string(file).unwrap();
            let articles: Vec<article::Article> = serde_json::from_str(&file_contents).unwrap();
            for article in articles.iter() {
                let words = Analyzer::split_abstract_into_words(article.paper_abstract.clone());
                analyzer.update_with_article_data(&words);
            }
            bar.inc(1);
        }
        analyzer.divide_rows_by_diagonal();
        bar.finish_with_message("Done building the relations matrix.");
    }

    fn detect_input_files(&mut self) {
        let mut counter = 1;
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
        let bar = indicatif::ProgressBar::new(self.filenames.len() as u64);
        bar.set_style(self.bar_style.clone());
        for file in self.filenames.clone().iter() {
            self.analyze_one_input_file(file.clone());
            bar.inc(1);
        }

        bar.finish_with_message("Done with computation.");
        print!(
            "Found a total of {} words.",
            self.keyword_candidates.len() as u32
        );

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
        let words = Analyzer::split_abstract_into_words(paper_abstract);
        for word in words {
            let counter = self.keyword_candidates.entry(word.to_string()).or_insert(0);
            *counter += 1;
        }
    }

    pub fn split_abstract_into_words(paper_abstract: String) -> Vec<String> {
        let mut cleared = paper_abstract.replace(".", " ");
        cleared = cleared.replace("?", " ");
        cleared = cleared.replace(";", " ");
        cleared = cleared.replace("(", " ");
        cleared = cleared.replace(")", " ");
        cleared = cleared.replace("!", " ");
        cleared = cleared.to_lowercase();
        let mut ret: Vec<String> = cleared.split_whitespace().map(|w| w.to_string()).collect();
        ret.sort();
        ret.dedup();
        ret
    }

    fn purge_keyword_array(&mut self) {
        let n_files = self.filenames.len() as f32;
        let lc = self.lower_cutoff * n_files;
        let uc = self.upper_cutoff * n_files;
        self.keyword_candidates
            .retain(|_, &mut count| (count as f32) > lc && (count as f32) < uc)
    }
}
