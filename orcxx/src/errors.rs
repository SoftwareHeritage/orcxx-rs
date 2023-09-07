// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

use std::fmt;

use thiserror::Error;

/// Wrapper for exceptions thrown by the underlying C++ library
#[derive(Debug)]
pub struct OrcError(pub cxx::Exception);

impl fmt::Display for OrcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for OrcError {}

impl From<cxx::Exception> for OrcError {
    fn from(exception: cxx::Exception) -> Self {
        OrcError(exception)
    }
}

impl PartialEq for OrcError {
    fn eq(&self, other: &Self) -> bool {
        self.what() == other.what()
    }
}

impl OrcError {
    pub fn what(&self) -> &str {
        self.0.what()
    }
}

pub type OrcResult<T> = Result<T, OrcError>;

#[derive(Error, Debug)]
pub enum OpenOrcError {
    #[error("Could not open ORC file for reading: {0}")]
    OrcError(OrcError),
    #[error("Unexpected ORC file type: {0}")]
    KindError(String),
}
