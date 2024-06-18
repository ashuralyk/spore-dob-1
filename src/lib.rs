#![cfg_attr(not(test), no_std)]

extern crate alloc;
pub mod decoder;
pub mod generated;

#[cfg(test)]
mod tests;
