// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

/// Wrapper for Apache ORC's Int128

#[cxx::bridge]
pub(crate) mod ffi {
    unsafe extern "C++" {
        include!("cpp-utils.hh");
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("orc/Int128.hh");

        type Int128;

        fn getHighBits(&self) -> i64;
        fn getLowBits(&self) -> u64;
    }
}
