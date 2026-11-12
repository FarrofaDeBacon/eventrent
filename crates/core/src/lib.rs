#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod domain;
pub mod error;
pub mod services;
pub mod store;
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub mod testing;

pub use domain::*;
pub use error::{CoreError, Result};
pub use services::*;
pub use store::*;
