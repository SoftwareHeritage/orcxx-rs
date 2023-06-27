// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

#[cxx::bridge]
pub(crate) mod ffi {
    unsafe extern "C++" {
        include!("cpp-utils.hh");
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type Int64DataBuffer;

        fn data(&self) -> *const i64;
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type StringDataBuffer;

        fn data(&self) -> *const *mut c_char;
    }
}
