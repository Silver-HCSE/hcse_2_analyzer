use histogram::Histogram;
use serde::{Deserialize, Serialize};
use sprs::{CsMat, CsVec};
use std::{collections::HashMap, io::Write};

use crate::{
    analyzer::{Analyzer, RatedPublication},
    DEFAULT_HALLMARKS,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hallmark {
    pub title: &'static str,
    pub description: &'static str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HallmarkRatingOutput {
    pub keyword: String,
    pub rating: Vec<f32>,
}

#[derive(Serialize, Debug)]
pub struct FullRunOutput {
    pub hallmarks: Vec<Hallmark>,
    pub rating_output: Vec<HallmarkRatingOutput>,
}

pub struct AnalyzerData {
    keywords_map: HashMap<String, usize>,
    relations: CsMat<f32>,
    keyword_ratings: Vec<CsVec<f32>>,
    n_keywords: usize,
    histogram: Histogram,
}

impl AnalyzerData {
    pub fn new(n_keywords: usize, keywords: &Vec<String>) -> AnalyzerData {
        let mut hm = HashMap::new();
        for word in keywords.iter().enumerate() {
            hm.entry(word.1.to_string()).or_insert(word.0);
        }
        let mut keyword_ratings = vec![];
        for _i in 0..DEFAULT_HALLMARKS.len() {
            let mut vec = CsVec::empty(n_keywords);
            for i in 0..n_keywords {
                vec.append(i, 0.0);
            }
            keyword_ratings.push(vec);
        }
        AnalyzerData {
            n_keywords,
            keywords_map: hm,
            relations: CsMat::zero((n_keywords, n_keywords)),
            keyword_ratings,
            histogram: Histogram::new(1, 32).unwrap(),
        }
    }

    pub fn print(&self) {
        println!(
            "Results of the analysis: Found {} keywords.",
            self.n_keywords
        );
        let n_matrix_entries = self.n_keywords * self.n_keywords;
        let percentage = ((self.relations.nnz() as f32 / n_matrix_entries as f32) * 100.0).floor();
        println!(
            "The matrix had a total of {} nonzero of {} total entries. {}%",
            self.relations.nnz(),
            n_matrix_entries,
            percentage
        );
        let buckets = self.histogram.as_slice();
        for b in 0..buckets.len() {
            if buckets[b] > 0 {
                println!("In bucket {}: {}", b, buckets[b]);
            }
        }
    }

    pub fn update_with_article_data(&mut self, words: &Vec<String>) {
        let mut present_keywords = vec![];
        for word in words.iter() {
            if self.keywords_map.contains_key(word) {
                present_keywords.push(word.clone());
            }
        }
        let indices: Vec<usize> = present_keywords
            .iter()
            .map(|w| *self.keywords_map.get(w).unwrap())
            .collect();
        let n_relevant_words = indices.len();
        let _ = self.histogram.increment(n_relevant_words as u64);
        for i in 0..n_relevant_words {
            let ind_i = indices[i];
            for j in i..n_relevant_words {
                let ind_j = indices[j];
                let current = self.relations.get(ind_i, ind_j).unwrap_or(&0.0).to_owned();
                let next = current + 1.0;
                self.relations.insert(ind_i, ind_j, next);
                self.relations.insert(ind_j, ind_i, next);
            }
        }
    }

    pub fn divide_rows_by_diagonal(&mut self) {
        let diag = self.relations.diag();
        for i in 0..self.n_keywords {
            for j in 0..self.n_keywords {
                let opt_val = self.relations.get_mut(i, j);
                match opt_val {
                    Some(val) => {
                        *val = *val / diag.get(i).unwrap_or(&1.0);
                    }
                    None => {}
                }
            }
        }
    }

    pub fn compute_keyword_ratings(&mut self) {
        for hallmark in DEFAULT_HALLMARKS.iter().enumerate() {
            let terms =
                Analyzer::split_abstract_into_words(hallmark.1.description.to_string(), true);
            for t in terms {
                if self.keywords_map.contains_key(&t) {
                    let keyword_index = *self.keywords_map.get(&t).unwrap();
                    let previous = self.keyword_ratings[hallmark.0][keyword_index];
                    self.keyword_ratings[hallmark.0][keyword_index] = previous + 1.0;
                }
            }
        }
        let n_unrated_keywords = self.normalize_keyword_rating();
        println!(
            "{} unrated keywords after initialization.",
            n_unrated_keywords
        );
        let n_max_update_steps = 1;
        for i in 0..n_max_update_steps {
            self.update_rating();
            let unrated_words = self.normalize_keyword_rating();
            println!("{} unrated keywords left in cycle {}", unrated_words, i);
        }
    }

    fn update_rating(&mut self) {
        for hallmark_index in 0..DEFAULT_HALLMARKS.len() {
            let mat: &CsMat<f32> = &self.relations;
            let vec: &CsVec<f32> = &self.keyword_ratings[hallmark_index];
            let new_rating = mat * vec;
            self.keyword_ratings[hallmark_index] = new_rating;
        }
    }

    pub fn write_rating_output(&self) {
        let mut rating_output: Vec<HallmarkRatingOutput> = vec![];
        for w in self.keywords_map.clone() {
            let mut rating: Vec<f32> = vec![];
            for i in 0..DEFAULT_HALLMARKS.len() {
                if self.is_rating_non_zero(w.1, i) {
                    rating.push(self.keyword_ratings[i][w.1]);
                } else {
                    rating.push(0.0);
                }
            }
            rating_output.push(HallmarkRatingOutput {
                keyword: w.0,
                rating,
            });
        }
        let full_output: FullRunOutput = FullRunOutput {
            hallmarks: DEFAULT_HALLMARKS.to_vec(),
            rating_output,
        };
        let output_json = serde_json::to_string_pretty(&full_output).unwrap();
        let mut file = std::fs::File::create("rating_database.json".to_string()).unwrap();
        file.write_all(output_json.as_bytes()).unwrap();
    }

    fn is_rating_non_zero(&self, word: usize, hallmark: usize) -> bool {
        self.keyword_ratings[hallmark].nnz_index(word).is_some()
    }

    fn normalize_keyword_rating(&mut self) -> usize {
        let mut number_of_unrated_words = 0;
        for i in 0..self.n_keywords {
            let mut sum = 0.0;
            for j in 0..DEFAULT_HALLMARKS.len() {
                if self.is_rating_non_zero(i, j) {
                    sum = sum + self.keyword_ratings[j][i];
                }
            }
            if sum > 0.0 {
                for j in 0..DEFAULT_HALLMARKS.len() {
                    if self.is_rating_non_zero(i, j) {
                        let current = self.keyword_ratings[j][i];
                        if current > 0.0 {
                            let new_value = current / sum;
                            self.keyword_ratings[j][i] = new_value;
                        }
                    }
                }
            } else {
                number_of_unrated_words = number_of_unrated_words + 1;
            }
        }
        number_of_unrated_words
    }

    pub fn rate_article_keywords(&self, words: Vec<String>, id: String) -> RatedPublication {
        let mut rating: Vec<f32> = vec![];
        let mut hm = HashMap::new();
        for word in words.iter().enumerate() {
            let counter = hm.entry(word.1.to_string()).or_insert(0);
            *counter += 1;
        }
        for _i in 0..DEFAULT_HALLMARKS.len() {
            rating.push(0.0);
        }

        let mut sum = 0.0;
        for word in hm {
            let keyword_index = self.keywords_map.get(&word.0).unwrap();
            for hallmark in 0..DEFAULT_HALLMARKS.len() {
                if self.is_rating_non_zero(*keyword_index, hallmark) {
                    let component =
                        self.keyword_ratings[hallmark][*keyword_index] * f32::sqrt(word.1 as f32);
                    rating[hallmark] += component;
                    sum += component;
                }
            }
        }

        for i in 0..DEFAULT_HALLMARKS.len() {
            rating[i] = rating[i] / sum;
        }
        RatedPublication {
            i: id.clone(),
            r: rating,
        }
    }
}
