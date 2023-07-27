//! Config structure
//!
//!

/*--- Use Statments ------------------------------------------------------------------------------*/

/*--- Const --------------------------------------------------------------------------------------*/

/*--- Types --------------------------------------------------------------------------------------*/

/*--- Implementation -----------------------------------------------------------------------------*/

use std::collections::{HashSet, HashMap};

use serde::{Deserialize, Serialize};

// pub mod default {
//     use super::*;
//     pub mod buttons {
//         use super::*;
//         pub fn
//     }
// }

structstruck::strike!{
    /// Configuration structure
    #[strikethrough[derive(Debug, Clone, Deserialize, Serialize)]]
    pub struct Config {
        /// Top level categories
        ///
        ///
        pub categories: HashSet<String>,

        /// Tags list
        ///
        /// All [Config::categories] can also be treated as tags
        pub tags: HashSet<String>,

        /// Default output folder
        pub default_folder: String,

        /// Button mappings
        ///
        /// HashMap of shortcut key (vim-like) to values
        pub buttons:
        HashMap<String,
        pub struct ButtonConfig {
            /// Long Label
            pub label: String,

            /// Button label
            pub button_label: String,

            /// Output path
            pub path: String,

            pub shortcut: char,
        }>,
    },
}

/*--------------------------------------------- EOF ----------------------------------------------*/
