//! Reading a Zig AIR dump, as a library.
//!
//! The binary is a thin front over these two modules: `air` turns the compiler's
//! text into instructions, `store` writes them as Parquet. They are public so
//! that the reader can be examined without running a compiler, which is the
//! only way to test it against dumps that no longer build.
pub mod air;
pub mod store;
