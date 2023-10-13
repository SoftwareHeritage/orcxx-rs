# v0.4.1

*2023-10-13*

Fixes:

* Make `orcxx_derive` depend on `orcxx` 0.4 instead of 0.3

# v0.4.0

*2023-10-13*

Breaking:

* Use `thiserror` instead of nested `Result`
* Rename 'utils' module to 'errors'

Additions:

* Add support for deserializing Timestamp as i64 struct field

Internal:

* Replace `unsafe_unwrap` with stdlib's `unwrap_unchecked`

# v0.3.0

*2023-08-25*

Breaking:

* Make `Reader::row_reader()` take a ref instead of owned RowReaderOptions

Additions:

* Add ParallelRowIterator
* Implement ExactSizeIterator and DoubleEndedIterator for RowIterator
* Implement `row_number()` and `seek_to_row()` for RowReader
* Impl Sync and Clone for RowReaderOptions
* Implement Send for vectors

Fixes: 

* Fix empty list deserialization

Internal:

* `orcxx_derive/tests/test1.rs`: Test more batch sizes (+deduplicate)

# v0.2.3

*2023-08-09*

Documentation:

* Avoid relative links between crate documentation


# v0.2.2

*2023-08-09*

Documentation:

* Copy examples to the README
* Add links between documentation pages

Internal:

* Add pre-commit config
* Remove orcxx/README.md symlink


# v0.2.1

*2023-08-09*

Documentation:

* `orcxx_derive`: Document RowIterator with an example

Internal:

* Use system libraries when building on docs.rs


# v0.2.0

*2023-08-08*

Breaking:

* RowIterator: Always check the selected kind
* Simplify RowIterator::new() to automatically select columns

Additions:

* `OrcStruct::columns()`
* Support for escaping field names

Internal:

* Fix dependencies between crates + dedup metadata


# v0.1.0

*2023-08-07*

Initial release.

Provides full read-only access to .orc files through three APIs:

* trees of vectors
* vectors of rows (structures generated with a custom derive)
* iterator on rows (ditto)

