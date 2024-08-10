// only use std when feature = "std" is enabled or during testing
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![feature(const_trait_impl)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(async_closure)]
#![feature(assert_matches)]
#![feature(never_type)]
#![allow(async_fn_in_trait)]

mod fmt;
mod vl_main;
pub mod avionics;
pub mod common;
pub mod driver;
mod gcm;
mod ground_test_avionics;
pub mod utils;
pub mod vacuum_test;

pub use common::device_manager::DeviceManager;
pub use common::console::rpc::RpcClient;
pub use vl_main::vl_main;