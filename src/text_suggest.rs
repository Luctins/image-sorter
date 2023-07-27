//! Suggestion engine

use std::{path::PathBuf, collections::{HashSet, HashMap}, io::prelude::*};

use cached::{proc_macro::cached, SizedCache};

/*--- Const --------------------------------------------------------------------------------------*/

pub const SEPARATOR: &'static str = "--";

/*--- Impl ---------------------------------------------------------------------------------------*/

/// Get search results
///
/// Cached
#[cached(
    type = "SizedCache<String, Vec<String>>",
    create = "{ SizedCache::with_size(50) }",
    convert = "{ prompt.to_string() }"
)]
pub fn hashset_search(dataset: &HashSet<String>, prompt: &str) -> Vec<String> {
    use rust_fuzzy_search::fuzzy_compare;
    use rust_fuzzy_search::fuzzy_search_sorted;

    fuzzy_search_sorted(
        prompt,
        &dataset
            .iter()
            .map(|item| item.as_str())
            .collect::<Vec<&str>>()
    )
        .iter()
        .map(|(item, _score)| String::from(*item))
        .collect()
}

pub fn get_segments<'s>(filename: &'s str) -> Vec<&'s str> {
    filename
        .split(SEPARATOR)
        .filter_map(|s| if ! s.is_empty() { Some(s) } else { None})
        .collect()
}

/*--------------------------------------------- EOF ----------------------------------------------*/
