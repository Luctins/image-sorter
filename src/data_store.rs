//!

use core::ops::{DerefMut, Deref};

use std::{io, path::PathBuf,};

use serde::{Deserialize, Serialize};

/*--- Impl ---------------------------------------------------------------------------------------*/

pub struct DataStore<D> {
    data: D,
    path: PathBuf,
}

impl<D> DataStore<D> where for<'d> D: Deserialize<'d> + Serialize {
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
