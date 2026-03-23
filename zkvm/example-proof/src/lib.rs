#![no_std]
/// Do not modify this file. This just serves to expose the generated
/// image ID and ELF bytes from the build script to the rest of the crate.
use core::{concat, env, include, include_bytes};
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
