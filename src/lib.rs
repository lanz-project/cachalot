pub use cachalot_proc_macro::cachalot;

mod idx_range;
pub use idx_range::IdxRange;

pub mod pages;

mod source;

mod config;
pub use config::*;

mod store;
pub use store::*;

pub type Idx = u128;
