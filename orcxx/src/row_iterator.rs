// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Iterator on ORC rows.
//!
//! Iterator items need to implement [`OrcDeserialize`] trait; `orcxx_derive` can
//! generate implementations for structures.
//!
//! TODO: write a test for this after we add the write API to vector batches
//! (currently it's only indirectly tested in `orcxx_derive`), because all the test
//! files have a structure at the root and we can't use `#[derive(OrcDeserialize)]`
//! in this crate to implement it.

use deserialize::{CheckableKind, OrcDeserialize, OrcStruct};
use reader::{Reader, RowReader, RowReaderOptions};
use std::convert::TryInto;
use std::num::NonZeroU64;
use utils::OrcError;
use vector::OwnedColumnVectorBatch;

/// Iterator on rows of the given [`RowReader`].
///
/// Reading from this may be less efficient than calling
/// [`OrcDeserialize::read_from_vector_batch`] and working on the column vector,
/// but provides a more familiar API to work with individual rows.
///
/// # Panics
///
/// next() repeatedly calls [`OrcDeserialize::read_from_vector_batch`] and panics
/// when it returns a [`::deserialize::DeserializationError`].
pub struct RowIterator<T: OrcDeserialize + Clone> {
    row_reader: RowReader,
    batch: OwnedColumnVectorBatch,
    decoded_batch: Vec<T>,

    /// Index in the decoded batch
    index: usize,

    /// Maximum value of the index + 1
    decoded_items: usize,

    /// Total number of lines in the file
    row_count: u64,
}

impl<T: OrcDeserialize + OrcStruct + CheckableKind + Clone> RowIterator<T> {
    /// Returns an iterator on rows of the given [`Reader`].
    ///
    /// This calls [`RowIterator::new_with_options`] with default options and
    /// includes only the needed columns (see [`RowReaderOptions::include_names`]).
    ///
    /// Errors are either detailed descriptions of format mismatch (as returned by
    /// [`CheckableKind::check_kind`], or C++ exceptions.
    ///
    /// # Panics
    ///
    /// When `batch_size` is larger than `usize`.
    pub fn new(
        reader: &Reader,
        batch_size: NonZeroU64,
    ) -> Result<Result<RowIterator<T>, String>, OrcError> {
        let options = RowReaderOptions::default().include_names(T::columns());
        Self::new_with_options(reader, batch_size, &options)
    }
}

impl<T: OrcDeserialize + Clone> RowIterator<T> {
    /// Returns an iterator on rows of the given [`RowReader`].
    ///
    /// Errors are detailed descriptions of format mismatch (as returned by
    /// [`CheckableKind::check_kind`].
    ///
    /// # Panics
    ///
    /// When `batch_size` is larger than `usize`.
    pub fn new_with_options(
        reader: &Reader,
        batch_size: NonZeroU64,
        options: &RowReaderOptions,
    ) -> Result<Result<RowIterator<T>, String>, OrcError> {
        let mut row_reader = reader.row_reader(options)?;
        match T::check_kind(&row_reader.selected_kind()) {
            Ok(_) => (),
            Err(msg) => return Ok(Err(msg)),
        }
        let batch_size: u64 = batch_size.into();
        let batch_size_usize = batch_size.try_into().expect("batch_size overflows usize");
        let mut decoded_batch = Vec::with_capacity(batch_size_usize);
        decoded_batch.resize_with(batch_size_usize, Default::default);
        Ok(Ok(RowIterator {
            batch: row_reader.row_batch(batch_size),
            row_reader,
            decoded_batch,
            index: 0,
            decoded_items: 0, // Will be filled on the first run of next()
            row_count: reader.row_count(),
        }))
    }

    pub fn seek(mut self, row_number: u64) -> Self {
        // TODO: avoid seeking in the underlying row_reader if the row we see is already
        // in the current buffer.
        self.row_reader.seek_to_row(row_number);
        self.index = 0;
        self.decoded_items = 0;
        self
    }
}

/// # Panics
///
/// next() repeatedly calls [`OrcDeserialize::read_from_vector_batch`] and panics
/// when it returns a [`::deserialize::DeserializationError`].
impl<T: OrcDeserialize + Clone> Iterator for RowIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        // Exhausted the current batch, read the next one.
        if self.index == self.decoded_items {
            self.index = 0;
            if !self.row_reader.read_into(&mut self.batch) {
                return None;
            }
            self.decoded_items = T::read_from_vector_batch(&self.batch.borrow(), &mut self.decoded_batch).expect("OrcDeserialize::read_from_vector_batch() call from RowIterator::next() returns a deserialization error");
        }

        let item = self.decoded_batch.get(self.index);
        self.index += 1;

        item.cloned()
    }
}

/// # Panics
///
/// next() repeatedly calls [`OrcDeserialize::read_from_vector_batch`] and panics
/// when it returns a [`::deserialize::DeserializationError`].
impl<T: OrcDeserialize + Clone> DoubleEndedIterator for RowIterator<T> {
    fn next_back(&mut self) -> Option<T> {
        // Exhausted the current batch, read the next one.
        if self.index == 0 {
            let row_number = self.row_reader.get_row_number();
            let batch_size: u64 = self
                .decoded_batch
                .len()
                .try_into()
                .expect("batch size overflowed u64");
            if row_number == 0 {
                return None;
            }
            let seek_to = row_number - u64::min(row_number, batch_size);
            self.row_reader.seek_to_row(seek_to);
            assert!(
                self.row_reader.read_into(&mut self.batch),
                "Rows {}..{} disappeared while rewinding",
                seek_to,
                row_number
            );
            self.decoded_items = T::read_from_vector_batch(&self.batch.borrow(), &mut self.decoded_batch).expect("OrcDeserialize::read_from_vector_batch() call from RowIterator::next_back() returns a deserialization error");
            self.index = self.decoded_items;
            assert_ne!(self.index, 0, "Got empty batch")
        }

        self.index -= 1;
        let item = self.decoded_batch.get(self.index);

        item.cloned()
    }
}

impl<T: OrcDeserialize + Clone> ExactSizeIterator for RowIterator<T> {
    fn len(&self) -> usize {
        let row_number = self.row_reader.get_row_number(); // number of the first row in the *current* batch
        if row_number == u64::MAX {
            // We didn't read anything yet
            self.row_count
                .try_into()
                .expect("row count overflows usize")
        } else {
            assert!(
                row_number <= self.row_count,
                "Iterated past the end (at row {})",
                row_number
            );
            let len_after_batch_start: usize = (self.row_count - row_number)
                .try_into()
                .expect("row count overflows usize");
            assert!(
                self.index <= len_after_batch_start,
                "Iterated past the end (index = {}, batch_start = {}, len_after_batch_start = {})",
                self.index,
                row_number,
                len_after_batch_start
            );
            len_after_batch_start - self.index
        }
    }
}
