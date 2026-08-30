//! Measuring how much source a task costs, as a library.
//!
//! The binary is a front over these modules. They are public so that the two
//! pieces with arithmetic in them — the line counter and the fit — can be
//! checked directly rather than through a corpus run.
pub mod corpus;
pub mod emit;
pub mod measure;
pub mod stats;
