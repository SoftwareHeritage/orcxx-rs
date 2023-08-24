// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Rayon-powered Iterator on ORC rows.
//!
//! Iterator items need to implement [`OrcDeserialize`] trait; `orcxx_derive` can
//! generate implementations for structures.
//!
//! TODO: write a test for this after we add the write API to vector batches
//! (currently it's only indirectly tested in `orcxx_derive`), because all the test
//! files have a structure at the root and we can't use `#[derive(OrcDeserialize)]`
//! in this crate to implement it.

use deserialize::{CheckableKind, OrcDeserialize, OrcStruct};
use reader::{Reader, RowReaderOptions};
use std::convert::TryInto;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::Arc;
use utils::OrcError;

use rayon::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback};
use rayon::prelude::*;

use row_iterator::RowIterator;

/// Parallel iterator on rows of the given [`Reader`].
///
/// Reading from this may be less efficient than calling
/// [`OrcDeserialize::read_from_vector_batch`] and working on the column vector,
/// but provides a more familiar API to work with individual rows.
///
/// # Panics
///
/// next() repeatedly calls [`OrcDeserialize::read_from_vector_batch`] and panics
/// when it returns a [`::deserialize::DeserializationError`].
pub struct ParallelRowIterator<T: OrcDeserialize + Clone> {
    reader: Arc<Reader>,
    row_reader_options: RowReaderOptions,
    batch_size: NonZeroU64,
    start: usize,
    end: usize,
    marker: PhantomData<T>,
}

impl<T: OrcDeserialize + OrcStruct + CheckableKind + Clone> ParallelRowIterator<T> {
    /// Returns a parallel iterator on rows of the given [`Reader`].
    ///
    /// This calls [`ParallelRowIterator::new_with_options`] with default options and
    /// includes only the needed columns (see [`RowReaderOptions::include_names`]).
    ///
    /// Errors are either detailed descriptions of format mismatch (as returned by
    /// [`CheckableKind::check_kind`], or C++ exceptions.
    ///
    /// # Panics
    ///
    /// When `batch_size` is larger than `usize`.
    pub fn new(
        reader: Arc<Reader>,
        batch_size: NonZeroU64,
    ) -> Result<Result<ParallelRowIterator<T>, String>, OrcError> {
        let options = RowReaderOptions::default().include_names(T::columns());
        Self::new_with_options(reader, batch_size, options)
    }
}

impl<T: OrcDeserialize + Clone> ParallelRowIterator<T> {
    /// Returns a parallel iterator on rows of the given [`Reader`].
    ///
    /// Errors are detailed descriptions of format mismatch (as returned by
    /// [`CheckableKind::check_kind`].
    ///
    /// # Panics
    ///
    /// When `batch_size` is larger than `usize`.
    pub fn new_with_options(
        reader: Arc<Reader>,
        batch_size: NonZeroU64,
        options: RowReaderOptions,
    ) -> Result<Result<ParallelRowIterator<T>, String>, OrcError> {
        match T::check_kind(&reader.row_reader(&options)?.selected_kind()) {
            Ok(_) => (),
            Err(msg) => return Ok(Err(msg)),
        }

        let row_count = reader
            .row_count()
            .try_into()
            .expect("row count overflows usize");
        Ok(Ok(ParallelRowIterator {
            reader: reader,
            row_reader_options: options,
            batch_size,
            start: 0,
            end: row_count,
            marker: PhantomData,
        }))
    }
}

impl<T: OrcDeserialize + Clone + Send + Sync> ParallelIterator for ParallelRowIterator<T> {
    type Item = T;

    fn drive_unindexed<C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>>(
        self,
        consumer: C,
    ) -> C::Result {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.end - self.start)
    }
}

impl<T: OrcDeserialize + Clone + Send + Sync> IndexedParallelIterator for ParallelRowIterator<T> {
    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        callback.callback(RowProducer {
            iter: &self,
            start: self.start,
            end: self.end,
        })
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn len(&self) -> usize {
        self.end - self.start
    }
}

struct RowProducer<'a, T: OrcDeserialize + Clone + Send + Sync> {
    iter: &'a ParallelRowIterator<T>,
    start: usize,
    end: usize,
}

impl<'a, T: OrcDeserialize + Clone + Send + Sync> Producer for RowProducer<'a, T> {
    type Item = T;
    type IntoIter = std::iter::Take<RowIterator<T>>;

    fn into_iter(self) -> Self::IntoIter {
        assert!(self.start <= self.end);
        let start = self
            .start
            .try_into()
            .expect("RowProducer::start overflows u64");
        RowIterator::new_with_options(
            &self.iter.reader,
            self.iter.batch_size,
            &self.iter.row_reader_options,
        )
        .expect("Could not create RowIterator") // Should be fine, was checked before
        .expect("Could not create RowIterator") // ditto
        .seek(start)
        .take(self.end - self.start) // TODO: tune the RowProducer buffer accordingly?
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        (
            RowProducer {
                iter: self.iter,
                start: self.start,
                end: self.start + index,
            },
            RowProducer {
                iter: self.iter,
                start: self.start + index,
                end: self.end,
            },
        )
    }
}
