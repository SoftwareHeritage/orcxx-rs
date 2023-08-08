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

