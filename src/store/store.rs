use std::hash::Hash;
use std::sync::Arc;

use async_trait::async_trait;

use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};

use tokio::fs::{create_dir_all, try_exists};

use bytemuck::{AnyBitPattern, NoUninit};

use crate::TypedConfig;
use crate::{Idx, IdxRange};

use crate::pages::PagesRange;
use crate::source::{Source, SourceRange};

use super::{cache_pages, load_pages, pages_dir, store_pages_range, PagesDir, StoreError};

#[async_trait]
pub trait Store<'a, R, K, V>: Source<R, K, V>
where
    R: SourceRange + Send + 'a,
    K: Send + Sync + Copy + Hash + 'a,
    V: NoUninit + AnyBitPattern + Send + Sync,
{
    async fn load<const PAGE_SIZE: Idx>(
        &'a self,
        k: K,
        r: R,
        config: &TypedConfig<V, PAGE_SIZE>,
    ) -> BoxStream<'a, V>
    where
        'a: 'async_trait,
    {
        match r.try_into() {
            Ok(range) => {
                let dir = pages_dir(k, config);

                let sealed = match try_exists(dir.as_ref()).await {
                    Ok(exists) => {
                        if exists {
                            self.load_or_cache::<PAGE_SIZE>(dir, k, (&range).into())
                                .boxed()
                        } else {
                            create_dir_all(dir.as_ref()).await.unwrap();

                            let pages = (&range).into();
                            let source = self.idx_range_source::<PAGE_SIZE>(k, range).await;

                            cache_pages::<_, PAGE_SIZE>(dir, pages, source)
                                .await
                                .boxed()
                        }
                    }
                    Err(err) => {
                        panic!(
                            "{}",
                            StoreError::<()>::PathAccess(err, dir.as_ref().clone())
                        );
                    }
                };

                sealed
                    .map(|result| match result {
                        Ok(item) => stream::iter(item),
                        Err(err) => panic!("{}", err),
                    })
                    .flatten()
                    .boxed()
            }
            Err(r) => self(k, r).await.boxed(),
        }
    }

    fn load_or_cache<const PAGE_SIZE: Idx>(
        &'a self,
        dir: PagesDir,
        k: K,
        pages: PagesRange<PAGE_SIZE>,
    ) -> BoxStream<'a, Result<Vec<V>, StoreError>> {
        async_stream::try_stream! {
            for await result in store_pages_range::<(), PAGE_SIZE>(dir.as_ref(), pages).await {
                let store_pages = result?;

                let pages_data = if store_pages.cached {
                    load_pages::<V, (), PAGE_SIZE>(Arc::clone(&dir), store_pages.pages).boxed()
                } else {
                    let pages = store_pages.pages.clone();
                    let source = self.pages_source(k, store_pages.pages).await;

                    cache_pages::<_, PAGE_SIZE>(Arc::clone(&dir), pages, source)
                        .await.boxed()
                };

                yield pages_data;
            }
        }
        .try_flatten()
        .boxed()
    }

    fn idx_range_source<const PAGE_SIZE: Idx>(&'a self, k: K, idx_range: IdxRange) -> Self::Fut {
        let range = idx_range.into();

        self(k, range)
    }

    fn pages_source<const PAGE_SIZE: Idx>(
        &'a self,
        k: K,
        pages: PagesRange<PAGE_SIZE>,
    ) -> Self::Fut {
        let idx_range: IdxRange = pages.into();

        self.idx_range_source::<PAGE_SIZE>(k, idx_range)
    }

    fn config<const PAGE_SIZE: Idx>(&self) -> TypedConfig<V, PAGE_SIZE> {
        TypedConfig::new()
    }
}

#[async_trait]
impl<'a, F, R, K, V> Store<'a, R, K, V> for F
where
    F: Source<R, K, V>,
    R: SourceRange + Send + 'a,
    K: Send + Sync + Copy + Hash + 'a,
    V: NoUninit + AnyBitPattern + Send + Sync,
{
}

#[cfg(test)]
mod tests {
    use std::ops::Range;
    use std::path::PathBuf;

    use tokio::fs::remove_dir_all;

    use super::*;

    #[tokio::test]
    async fn test_load() {
        async fn source(_k: &(), range: Range<Idx>) -> BoxStream<'static, Idx> {
            stream::iter(range).boxed()
        }

        const PAGE_SIZE: Idx = 1024;

        let mut config = source.config::<PAGE_SIZE>();
        config.root = PathBuf::from(format!("{}", rand::random::<u128>())).into();

        let range = 512..4096 as Idx;

        let values_1 = source
            .load::<PAGE_SIZE>(&(), range.clone(), &config)
            .await
            .collect::<Vec<_>>()
            .await;

        let values_2 = source
            .load::<PAGE_SIZE>(&(), range, &config)
            .await
            .collect::<Vec<_>>()
            .await;

        assert_eq!(values_1, values_2);

        remove_dir_all(&config.root).await.unwrap();
    }
}
