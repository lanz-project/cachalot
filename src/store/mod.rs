use std::borrow::Cow;
use std::fs::File;
use std::hash::{BuildHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use thiserror::Error;

use futures::pin_mut;
use futures::stream::{Stream, StreamExt, TryStreamExt};

use tokio::task;

use bytemuck::{AnyBitPattern, NoUninit};

use crate::pages::{PageRange, PagesRange};
use crate::{Idx, TypedConfig};

mod store;
pub use store::*;

mod try_store;
pub use try_store::*;

mod store_pages;
pub(super) use store_pages::*;

type PagesDir = Arc<Cow<'static, Path>>;
type PagePath = Cow<'static, Path>;

#[derive(Debug, Error)]
pub enum StoreError<E = ()> {
    #[error("File creation error - path: {1}; io-error: {0}")]
    FileCreation(std::io::Error, Cow<'static, Path>),
    #[error("File open error - path: {1}; io-error: {0}")]
    FileOpen(std::io::Error, Cow<'static, Path>),
    #[error("Page write error - path: {0}")]
    PageWrite(Cow<'static, Path>),
    #[error("Page read error - path: {0}")]
    PageRead(Cow<'static, Path>),
    #[error("Path access error - path: {1}; io-error: {0}")]
    PathAccess(std::io::Error, Cow<'static, Path>),
    #[error("external error")]
    External(E),
}

fn load_pages<'a, V, E, const PAGE_SIZE: Idx>(
    dir: PagesDir,
    pages: PagesRange<PAGE_SIZE>,
) -> impl Stream<Item = Result<Vec<V>, StoreError<E>>> + 'a
where
    V: NoUninit + AnyBitPattern + Send,
    E: Send + 'static,
{
    async_stream::stream! {
        for page in pages.pages() {
            let page_path = page_path(dir.as_ref(), &page.page);

            yield load_page(page_path, page).await;
        }
    }
}

async fn load_page<'a, V, E, const PAGE_SIZE: Idx>(
    page_path: PagePath,
    page: PageRange<PAGE_SIZE>,
) -> Result<Vec<V>, StoreError<E>>
where
    V: NoUninit + AnyBitPattern + Send,
    E: Send + 'static,
{
    use std::io::{Read, Seek, SeekFrom};
    use std::mem::size_of;

    use bytemuck::cast_slice_mut;

    let join = task::spawn_blocking(move || {
        let mut file = File::open(page_path.as_ref())
            .map_err(|err| StoreError::FileOpen(err, page_path.clone()))?;
        file.seek(SeekFrom::Start(page.first as u64 * size_of::<V>() as u64))
            .map_err(|_| StoreError::PageRead(page_path.clone()))?;

        let mut buf = vec![V::zeroed(); page.len()];

        file.read(cast_slice_mut(&mut buf))
            .map_err(|_| StoreError::PageRead(page_path.clone()))?;

        Ok(buf)
    });

    match join.await {
        Err(err) => {
            panic!("{}", err)
        }
        Ok(data) => data,
    }
}

pub(super) async fn cache_pages<'a, V, const PAGE_SIZE: Idx>(
    dir: PagesDir,
    pages: PagesRange<PAGE_SIZE>,
    source: impl Stream<Item = V> + 'a,
) -> impl Stream<Item = Result<Vec<V>, StoreError>> + 'a
where
    V: NoUninit + Send,
{
    async_stream::stream! {
        pin_mut!(source);

        for page in pages.pages() {
            let page_data = source.by_ref().take(page.len()).collect::<Vec<_>>().await;

            yield cache_page(dir.as_ref(), page, page_data).await;
        }
    }
}

pub(super) async fn try_cache_pages<'a, V, E, const PAGE_SIZE: Idx>(
    dir: PagesDir,
    pages: PagesRange<PAGE_SIZE>,
    source: impl Stream<Item = Result<V, E>> + 'a,
) -> impl Stream<Item = Result<Vec<V>, StoreError<E>>> + 'a
where
    V: NoUninit + Send,
    E: Send + 'static,
{
    async_stream::try_stream! {
        pin_mut!(source);

        for page in pages.pages() {
            let page_data = source
                .by_ref()
                .take(page.len())
                .try_collect::<Vec<_>>()
                .await
                .map_err(|err| StoreError::External(err))?;

            yield cache_page(dir.as_ref(), page, page_data).await?;
        }
    }
}

pub(super) async fn cache_page<V, E, const PAGE_SIZE: Idx>(
    dir: impl AsRef<Path>,
    page: PageRange<PAGE_SIZE>,
    data: Vec<V>,
) -> Result<Vec<V>, StoreError<E>>
where
    V: NoUninit + Send,
    E: Send + 'static,
{
    use std::io::Write;

    use bytemuck::cast_slice;

    if page.full_fill() {
        let path = page_path(dir.as_ref(), &page.page);

        let join = task::spawn_blocking(move || {
            let mut file =
                File::create(&path).map_err(|err| StoreError::FileCreation(err, path.clone()))?;

            file.write_all(cast_slice(&data[..]))
                .map_err(|_| StoreError::PageWrite(path.clone()))?;

            Ok::<_, StoreError<E>>(data)
        });

        match join.await {
            Err(err) => {
                panic!("{}", err)
            }
            Ok(data) => data,
        }
    } else {
        Ok(data)
    }
}

pub(super) fn page_path(dir: impl AsRef<Path>, page: &Idx) -> PagePath {
    dir.as_ref().join(format!("{}", page)).into()
}

pub(super) fn pages_dir<K: Hash, V: 'static, const PAGE_SIZE: Idx>(
    k: K,
    config: &TypedConfig<V, PAGE_SIZE>,
) -> PagesDir {
    use std::any::TypeId;
    use std::mem::size_of;

    use ahash::RandomState;

    const STATE: RandomState = RandomState::with_seeds(
        2858199611238995053,
        11086922458483105823,
        15118288254885688199,
        573726014207964826,
    );

    let mut hasher = STATE.build_hasher();

    TypeId::of::<TypedConfig<V, PAGE_SIZE>>().hash(&mut hasher);

    size_of::<V>().hash(&mut hasher);

    k.hash(&mut hasher);

    Arc::new(config.root.join(format!("{}", hasher.finish())).into())
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;
    use std::path::PathBuf;

    use futures::stream::{self, BoxStream};

    use tokio::fs::{create_dir, remove_dir_all, try_exists};

    use crate::pages::PagesRange;
    use crate::{Idx, IdxRange};

    use super::*;

    #[tokio::test]
    async fn test_cache_pages() {
        async fn source(_k: &(), range: RangeInclusive<Idx>) -> BoxStream<'static, Idx> {
            stream::iter(range).boxed()
        }

        const PAGE_SIZE: Idx = 1024;

        let range = 512..=4095 as Idx;

        let dir: Arc<Cow<'static, _>> =
            Arc::new(PathBuf::from(format!("{}", rand::random::<u128>())).into());
        create_dir(dir.as_ref()).await.unwrap();

        let idx_range = IdxRange::try_from(range.clone()).unwrap();

        let values = cache_pages::<_, PAGE_SIZE>(
            dir.clone(),
            PagesRange::<PAGE_SIZE>::from(&idx_range),
            source(&(), range.clone()).await,
        )
        .await
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .map(IntoIterator::into_iter)
        .flatten()
        .collect::<Vec<_>>();

        assert_ne!(try_exists(dir.join("0")).await.unwrap(), true);
        assert_eq!(try_exists(dir.join("1")).await.unwrap(), true);
        assert_eq!(try_exists(dir.join("2")).await.unwrap(), true);
        assert_eq!(try_exists(dir.join("3")).await.unwrap(), true);

        assert_eq!(range.collect::<Vec<_>>(), values);

        remove_dir_all(dir.as_ref()).await.unwrap();
    }

    #[tokio::test]
    async fn test_load_pages() {
        async fn source(_k: &(), range: RangeInclusive<Idx>) -> BoxStream<'static, Idx> {
            stream::iter(range).boxed()
        }

        const PAGE_SIZE: Idx = 1024;

        let range = 0..=((2 as Idx).pow(11) - 1);

        let dir: Arc<Cow<'static, _>> =
            Arc::new(PathBuf::from(format!("{}", rand::random::<u128>())).into());
        create_dir(dir.as_ref()).await.unwrap();

        let idx_range = IdxRange::try_from(range.clone()).unwrap();

        let values = cache_pages::<_, PAGE_SIZE>(
            dir.clone(),
            PagesRange::<PAGE_SIZE>::from(&idx_range),
            source(&(), range.clone()).await,
        )
        .await
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .map(IntoIterator::into_iter)
        .flatten()
        .collect::<Vec<_>>();

        assert_eq!(range.clone().collect::<Vec<_>>(), values);

        let offset = 512;
        let cached_range = IdxRange::try_from(offset..(2 as Idx).pow(11)).unwrap();
        let cached = load_pages::<Idx, (), PAGE_SIZE>(
            Arc::clone(&dir),
            PagesRange::<PAGE_SIZE>::from(&cached_range),
        )
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .map(IntoIterator::into_iter)
        .flatten()
        .collect::<Vec<_>>();

        assert_eq!(
            range.skip(offset as usize).collect::<Vec<_>>().len(),
            cached.len()
        );

        remove_dir_all(dir.as_ref()).await.unwrap();
    }
}
