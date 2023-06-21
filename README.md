# orcxx-rs

Unofficial Rust binding for the official C++ library for Apache ORC.

It uses a submodule pointing to an Apache ORC release, builds its C++ part
(including vendored protobuf, lz4, zstd, ...), and links against that.

