// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

use cxx::{let_cxx_string, UniquePtr};

use kind;
use utils::{OrcError, OrcResult};
use vector;

#[cxx::bridge]
pub(crate) mod ffi {
    #[namespace = "orcxx_rs::utils"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");
        include!("orc/OrcFile.hh");

        #[rust_name = "ReaderOptions_new"]
        fn construct() -> UniquePtr<ReaderOptions>;

        #[rust_name = "RowReaderOptions_new"]
        fn construct() -> UniquePtr<RowReaderOptions>;
    }

    // Reimport types from other modules
    #[namespace = "orc"]
    unsafe extern "C++" {
        type ColumnVectorBatch = crate::vector::ffi::ColumnVectorBatch;
        type Type = crate::kind::ffi::Type;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type InputStream;
        type ReaderOptions;
        type RowReaderOptions;

        fn readLocalFile(path: &CxxString) -> Result<UniquePtr<InputStream>>;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type Reader;

        fn createReader(
            inStream: UniquePtr<InputStream>,
            options: &ReaderOptions,
        ) -> UniquePtr<Reader>;

        fn createRowReader(&self, rowReaderOptions: &RowReaderOptions) -> UniquePtr<RowReader>;

        fn getType(&self) -> &Type;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type RowReader;

        fn createRowBatch(&self, size: u64) -> UniquePtr<ColumnVectorBatch>;

        fn next(self: Pin<&mut RowReader>, data: Pin<&mut ColumnVectorBatch>) -> bool;

        fn getSelectedType(&self) -> &Type;
    }
}

pub struct ReaderOptions(UniquePtr<ffi::ReaderOptions>);

impl Default for ReaderOptions {
    fn default() -> ReaderOptions {
        ReaderOptions(ffi::ReaderOptions_new())
    }
}

pub struct InputStream(UniquePtr<ffi::InputStream>);

impl InputStream {
    pub fn from_local_file(file_name: &str) -> OrcResult<InputStream> {
        let_cxx_string!(cxx_file_name = file_name);
        ffi::readLocalFile(&cxx_file_name)
            .map(InputStream)
            .map_err(OrcError)
    }
}

pub struct Reader(UniquePtr<ffi::Reader>);

impl Reader {
    pub fn new(input_stream: InputStream) -> Reader {
        Reader::new_with_options(input_stream, ReaderOptions::default())
    }

    pub fn new_with_options(input_stream: InputStream, options: ReaderOptions) -> Reader {
        Reader(ffi::createReader(input_stream.0, &*options.0))
    }

    pub fn row_reader(&self, options: RowReaderOptions) -> RowReader {
        RowReader(self.0.createRowReader(&options.0))
    }

    /// Returns the data type of the file being read. This is usually a struct.
    pub fn kind(&self) -> kind::Kind {
        kind::Kind::new_from_orc_type(self.0.getType())
    }
}

pub struct RowReaderOptions(UniquePtr<ffi::RowReaderOptions>);

impl Default for RowReaderOptions {
    fn default() -> RowReaderOptions {
        RowReaderOptions(ffi::RowReaderOptions_new())
    }
}

pub struct RowReader(UniquePtr<ffi::RowReader>);

impl RowReader {
    pub fn row_batch(&mut self, size: u64) -> vector::OwnedColumnVectorBatch {
        vector::OwnedColumnVectorBatch(self.0.createRowBatch(size))
    }

    /// Read the next stripe into the batch, or returns false if there are no
    /// more stripes.
    pub fn read_into(&mut self, batch: &mut vector::OwnedColumnVectorBatch) -> bool {
        self.0.pin_mut().next(batch.0.pin_mut())
    }

    /// Returns the data type being read.
    ///
    /// With the default [RowReaderOptions], this is the same as [Reader::kind].
    /// Otherwise this is usually a subset [Reader::kind].
    pub fn selected_kind(&self) -> kind::Kind {
        kind::Kind::new_from_orc_type(self.0.getSelectedType())
    }
}
