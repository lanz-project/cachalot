use std::borrow::Cow;
use std::marker::PhantomData;
use std::path::Path;

use derive_more::{Deref, DerefMut};

use crate::Idx;

#[derive(Hash)]
pub struct Config<const PAGE_SIZE: Idx> {
    pub root: Cow<'static, Path>,
}

impl<const PAGE_SIZE: Idx> Config<PAGE_SIZE> {
    pub const DEFAULT_ROOT: &'static str = ".cachalot";

    pub fn new() -> Self {
        Config {
            root: Path::new(Self::DEFAULT_ROOT).into(),
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct TypedConfig<V, const PAGE_SIZE: Idx> {
    #[deref]
    #[deref_mut]
    pub config: Config<PAGE_SIZE>,
    _type: PhantomData<V>,
}

impl<V, const PAGE_SIZE: Idx> TypedConfig<V, PAGE_SIZE> {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
            _type: PhantomData,
        }
    }
}
