//! Suggestion engine

use std::{path::PathBuf, collections::{HashSet, HashMap}, io::prelude::*};

use cached::{proc_macro::cached, SizedCache};

use crate::*;

/*--- Const --------------------------------------------------------------------------------------*/
pub const TAG_SEPARATOR: &'static str = "--";

pub const CATEGORIES_FILE_NAME: &'static str = "categories.json";

pub use super::DEFAULT_CATEGORIES_S;

lazy_static::lazy_static!{
    static ref DEFAULT_CATEGORIES:HashSet<String> = serde_json::from_str(DEFAULT_CATEGORIES_S)
        .expect("failed to parse default configuration");
}

/*--- Impl ---------------------------------------------------------------------------------------*/


/// Text suggestion engine
pub struct TextSuggester {
    /// text completion selection
    pub current_selection: Option<usize>,
    pub new_category_buffer: String,

    categories: HashSet<String>,
    config_path: PathBuf,
}

impl TextSuggester {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        let _config_path = config_path.as_ref();
        let config_path = _config_path.join(CATEGORIES_FILE_NAME);
        println!("categories config path: {config_path:?}");

        Self {
            //last_key: None,
            current_selection: None,
            categories: {
                use std::io::ErrorKind;

                match std::fs::read_to_string(&config_path) {
                    Ok(categories) => {
                        println!("categories: {categories}");
                        serde_json::from_str(&categories)
                            .expect("cannot parse categories preference file")
                    },

                    Err(e) => {
                        match e.kind() {
                            ErrorKind::NotFound => {
                                print!("no configuration found, using default ");
                                println!("and copying default config file to ");

                                crate::copy_default(&config_path, &DEFAULT_CATEGORIES_S);
                                DEFAULT_CATEGORIES.clone()
                            },

                            _ => {
                                panic!("failed to read preference file: {e}");
                            }
                        }
                    }
                }
            },
            new_category_buffer: String::new(),
            config_path,
        }
    }

    pub fn get_suggestions(&mut self, prompt: &str) -> Vec<String> {
        get_results(&self.categories, prompt)
    }


    // pub fn last_key_changed(&mut self, key: egui::Key) -> bool {
    //     if let Some(ref mut last_key) = self.last_key {
    //         if *last_key != key {
    //             *last_key = key;
    //             true
    //         } else {
    //             false
    //         }
    //     } else {
    //         self.last_key = Some(key);
    //         true
    //     }
    // }

    pub fn add_category(&mut self) {
        self.categories.insert(self.new_category_buffer.clone());
        println!("added new category: {}", self.new_category_buffer);
        self.new_category_buffer.clear();

        let cfg_str = serde_json::to_string(&self.categories).unwrap();
        std::fs::write(&self.config_path, &cfg_str).expect("cannot write preferences");
    }
}

/// get results
///
/// Outside because of caching method cannot have self as arg
#[cached(
    type = "SizedCache<String, Vec<String>>",
    create = "{ SizedCache::with_size(20) }",
    convert = "{ prompt.to_string() }"
)]
fn get_results(categories: &HashSet<String>, prompt: &str) -> Vec<String> {
    use rust_fuzzy_search::fuzzy_compare;
    use rust_fuzzy_search::fuzzy_search_sorted;

    fuzzy_search_sorted(prompt, &categories.iter().map(|item| item.as_str()).collect::<Vec<&str>>())
        .iter()
        .map(|(cat, _score)| String::from(*cat))
        .collect()

    // categories
    //     .iter()
    //     .filter_map(|cat| {
    //         let score = fuzzy_compare(cat, prompt);
    //         //println!("score: {score}");

    //         if score > 0.0 {
    //             Some(score, cat.clone())
    //         } else {
    //             None
    //         }
    //     })
    //     .collect()
}


pub fn get_segments<'s>(filename: &'s str) -> Vec<&'s str> {
    filename
        .split(TAG_SEPARATOR)
        .filter_map(|s| if ! s.is_empty() { Some(s) } else { None})
        .collect()
}
