use std::path::Path;

use bool_ext::BoolExt;

use accumulable::try_stream::TryPartiallyAccumulate;
use accumulable::{Accumulable, MaybeAccumulable};

use futures::Stream;

use tokio::fs::try_exists;

use crate::pages::PagesRange;
use crate::{Idx, StoreError};

pub struct StorePages<const PAGE_SIZE: Idx> {
    pub cached: bool,
    pub pages: PagesRange<PAGE_SIZE>,
}

impl<const PAGE_SIZE: Idx> MaybeAccumulable for StorePages<PAGE_SIZE> {
    fn maybe_accumulate_from(&mut self, rhs: &Self) -> bool {
        (self.cached == rhs.cached).and_do(|| self.pages.accumulate_from(&rhs.pages))
    }
}

pub async fn store_pages_range<E, const PAGE_SIZE: Idx>(
    dir: impl AsRef<Path>,
    pages: PagesRange<PAGE_SIZE>,
) -> impl Stream<Item = Result<StorePages<PAGE_SIZE>, StoreError<E>>> {
    async_stream::stream! {
        for page in pages.pages() {
            let file = super::page_path(&dir, &page.page);

            yield try_exists(&file)
                .await
                .map(|exists| StorePages {
                    cached: exists,
                    pages: page.into(),
                })
                .map_err(|err| StoreError::PathAccess(err, file));
        }
    }
    .try_partially_accumulate()
}
