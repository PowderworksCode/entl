mod bun;
mod cargo;
mod npm;
mod pnpm;
mod yarn;

pub use bun::PROFILE as BUN;
pub use cargo::PROFILE as CARGO;
pub use npm::PROFILE as NPM;
pub use pnpm::PROFILE as PNPM;
pub use yarn::PROFILE as YARN;
