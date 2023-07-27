//!

#![cfg_attr(debug_assertions, allow(unused))]

// TODO: later allow choosing the deserializer

/*--- Use ----------------------------------------------------------------------------------------*/

use core::ops::{DerefMut, Deref};

use std::{io, path::{PathBuf, Path}, fs::OpenOptions,};

use serde::{Deserialize, Serialize};

/*--- Impl ---------------------------------------------------------------------------------------*/

// pub type DataSer<D> = fn(&D) -> Result<String, E>;
// pub type DataDeser<D> = fn(&str) -> Result<D, ()>;


pub struct DataStore<D> {
    data: D,
    path: PathBuf,
}

type DataStoreError = serde_yaml::Error;

impl<D> DataStore<D> where for<'d> D: Deserialize<'d> + Serialize {

    /// new from path
    ///
    /// Using default deserializer
    // TODO: add error type
    pub fn new_from_path(source: &Path) -> Result<Self, ()> {
        let mut f = OpenOptions::new()
            .read(true)
            .open(source)
            .map_err(|e| eprintln!("error: {e}"))?;

        let s = Self {
            data: serde_yaml::from_reader(&mut f)
                .map_err(|e| eprintln!("serde yaml error: {e}"))?,
            path: source.to_path_buf(),
            // deserializer,
            // serializer,
        };
        Ok(s)
        // Self::new_with_deserializer(source, serde_yaml::to_string::<D>, serde_yaml::from_str::<D>)
    }

    pub fn new_from_data(source: &Path, data: D) -> Self {
        Self {
            path: source.to_path_buf(),
            data,
        }
    }

    // pub fn new_with_deserializer(
    //     source: &Path,
    //     serializer: DataSer<D>,
    //     deserializer: DataDeser<D>
    // ) -> Result<Self, ()> {
    // }

    pub fn save(&mut self) -> io::Error {
        todo!()
    }
}

impl<D> Deref for DataStore<D> {
    type Target = D;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<D> DerefMut for DataStore<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/*--------------------------------------------- EOF ----------------------------------------------*/
