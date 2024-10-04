use crate::analyzer_data::AnalyzerData;
use crate::{article, DEFAULT_HALLMARKS};
use serde::ser::{SerializeSeq, Serializer};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::{collections::HashMap, io::Write};

fn serialize_f32_vec<S>(vec: &Vec<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(vec.len()))?;
    for &num in vec.iter() {
        seq.serialize_element(&format!("{:.3}", num))?;
    }
    seq.end()
}

pub struct Analyzer {
    filenames: Vec<String>,
    keyword_candidates: HashMap<String, usize>,
    lower_cutoff: f32,
    upper_cutoff: f32,
    bar_style: indicatif::ProgressStyle,
}

#[derive(Serialize, Debug)]
pub struct RatedPublication {
    pub i: String,
    #[serde(serialize_with = "serialize_f32_vec")]
    pub r: Vec<f32>,
}

impl RatedPublication {
    pub fn is_valid(&self) -> bool {
        let mut rating_norm = 0.0;
        for i in 0..DEFAULT_HALLMARKS.len() {
            rating_norm += self.r[i];
        }

        rating_norm > 0.95 && rating_norm < 1.05
    }
}
impl Analyzer {
    pub fn new(lower_cutoff: f32, upper_cutoff: f32) -> Self {
        let bar_style = indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>5}/{len:5} {msg} {eta}",
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
        self.rate_publications(analyzer_data);
    }

    fn rate_publications(&self, analyzer: AnalyzerData) {
        let mut article_ratings = vec![];
        let bar = indicatif::ProgressBar::new(self.filenames.len() as u64);
        bar.set_message("Rating the article database.");
        bar.set_style(self.bar_style.clone());
        for file in self.filenames.iter() {
            let file_contents: String = fs::read_to_string(file).unwrap();
            let articles: Vec<article::Article> = serde_json::from_str(&file_contents).unwrap();
            for article in articles.iter() {
                if article.pmc != "" {
                    let words =
                        Analyzer::split_abstract_into_words(article.paper_abstract.clone(), false);
                    let article_rating: RatedPublication =
                        analyzer.rate_article_keywords(words, article.pmc.clone());
                    if article_rating.is_valid() {
                        article_ratings.push(article_rating);
                    }
                }
            }
            bar.inc(1);
        }

        bar.finish_with_message("Done rating publications.");

        println!("Rated a total of {} articles.", article_ratings.len());
        let output_json = serde_json::to_string(&article_ratings).unwrap();
        let mut file = std::fs::File::create("article_database.json".to_string()).unwrap();
        file.write_all(output_json.as_bytes()).unwrap();
    }

    fn build_relations_matrix(&self, analyzer: &mut AnalyzerData) {
        let bar = indicatif::ProgressBar::new(self.filenames.len() as u64);
        bar.set_message("Building Relations Matrix");
        bar.set_style(self.bar_style.clone());
        for file in self.filenames.iter() {
            let file_contents: String = fs::read_to_string(file).unwrap();
            let articles: Vec<article::Article> = serde_json::from_str(&file_contents).unwrap();
            for article in articles.iter() {
                let words =
                    Analyzer::split_abstract_into_words(article.paper_abstract.clone(), true);
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
        bar.set_message("Searching for possible keywords...");
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
        let words = Analyzer::split_abstract_into_words(paper_abstract, true);
        for word in words {
            let counter = self.keyword_candidates.entry(word.to_string()).or_insert(0);
            *counter += 1;
        }
    }

    pub fn split_abstract_into_words(paper_abstract: String, dedupe: bool) -> Vec<String> {
        let re = Regex::new(r#"[.?,;()!\/'"%=]"#).unwrap();
        let cleared = re.replace_all(&paper_abstract, " ").to_string().to_lowercase();
        let mut ret: Vec<String> = cleared.split_whitespace().map(|w| Analyzer::clean_keyword(w.to_string())).collect();
        ret.retain(|w| w.len() > 4);
        ret.sort();
        if dedupe {
            ret.dedup();
        }
        ret
    }

    pub fn clean_keyword(in_word: String) -> String {
        let mut ret = in_word.clone().to_string();
        let mut has_changed = true;
        while has_changed {
            has_changed = false;
            if ret.len() > 4 {
            let first_char = ret.chars().next().unwrap();
            if first_char == '-' {
                ret.remove(0);
                has_changed = true;
            }
            let last_char: char = ret.chars().last().unwrap();
            if last_char == '-' {
                ret.pop();
                has_changed = true;
            }
            }
        }
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
