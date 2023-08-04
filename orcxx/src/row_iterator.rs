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

use deserialize::OrcDeserialize;
use reader::RowReader;
use std::convert::TryInto;
use std::num::NonZeroU64;
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
}

impl<T: OrcDeserialize + Clone> RowIterator<T> {
    /// Returns an iterator on rows of the given [`RowReader`].
    ///
    /// # Panics
    ///
    /// When `batch_size` is larger than `usize`.
    pub fn new(mut row_reader: RowReader, batch_size: NonZeroU64) -> RowIterator<T> {
        let batch_size: u64 = batch_size.into();
        let batch_size_usize = batch_size.try_into().expect("batch_size overflows usize");
        let mut decoded_batch = Vec::with_capacity(batch_size_usize);
        decoded_batch.resize_with(batch_size_usize, Default::default);
        RowIterator {
            batch: row_reader.row_batch(batch_size),
            row_reader,
            decoded_batch,
            index: 0,
            decoded_items: 0, // Will be filled on the first run of next()
        }
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
            if !self.row_reader.read_into(&mut self.batch) {
                return None;
            }
            self.decoded_items = T::read_from_vector_batch(&self.batch.borrow(), &mut self.decoded_batch).expect("OrcDeserialize::read_from_vector_batch() call from RowIterator::next() returns a deserialization error");
            self.index = 0;
        }

        let item = self.decoded_batch.get(self.index);
        self.index += 1;

        item.cloned()
    }
}
