#![feature(plugin)]
#![recursion_limit = "1024"]
#![allow(dead_code, unused_must_use, unused_variables)]

extern crate byteorder;
pub extern crate rmp;
pub extern crate rmp_serde;
extern crate serde;
extern crate serde_bytes;
extern crate tempdir;
extern crate tempfile;
extern crate time;

#[macro_use]
extern crate error_chain;

#[macro_use]
pub mod util;

#[macro_use]
pub mod msg;

pub mod errors;
pub mod storage;
mod index;
mod lock;
mod pool;
mod records;
pub mod reader;
pub mod writer;
pub mod tid;
mod transaction;
