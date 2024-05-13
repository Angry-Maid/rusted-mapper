#![feature(duration_constructors)]

use std::time::Duration;

use rm_core::parser::parse;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    let t = parse("14:25:37.123".as_bytes());
    if let Ok(t) = t {
        let v = (t + Duration::from_mins(14) + Duration::from_millis(321)) - t;
        println!(
            "{:?} {:?} {:?} {:?}",
            t,
            t + Duration::from_hours(3),
            t + Duration::from_mins(14) + Duration::from_millis(321),
            v
        );
    } else if let Err(t) = t {
        println!("{:?}", t);
    }

    println!(
        "{} {}{} - compiler {}",
        built_info::PKG_VERSION,
        built_info::GIT_COMMIT_HASH_SHORT.unwrap(),
        if built_info::GIT_DIRTY.unwrap() {
            "(dirty)"
        } else {
            ""
        },
        built_info::RUSTC_VERSION
    );
}
