// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

/// Reformats line-separated JSON to look like the output of to_json.
///
/// This does not operate on ORC files, and is only meant as a debugging helper.
use std::io;

fn main() {
    for line in io::stdin().lines() {
        println!(
            "{}",
            json::stringify_pretty(
                json::parse(line.as_ref().expect(&format!("Could not read line")))
                    .expect(&format!("Could not parse {:?} as JSON", line)),
                4
            )
        );
    }
}
