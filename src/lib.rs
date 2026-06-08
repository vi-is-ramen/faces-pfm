#![cfg_attr(feature = "no_std", no_std)]
#![feature(const_clone)]
#![feature(const_default)]
#![feature(const_trait_impl)]

//! # `faces-pfm`
//!
//! A reference implementation of the `faces::AbsPageFrameManager` trait.
//!
//! This crate provides a concrete page frame manager for the `faces` memory
//! management framework, including per-frame metadata, flag handling, and
//! integration with the Limine boot protocol for memory map discovery.

pub mod flags;
pub mod frame;
pub mod mgr;

pub use flags::PageFlags;
pub use frame::PageFrame;
pub use mgr::PageFrameManager;
