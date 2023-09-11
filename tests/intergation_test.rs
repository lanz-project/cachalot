#[test]
fn ui() {
    let t = trybuild::TestCases::new();

    t.compile_fail("tests/ui/*.rs");
}

#[tokio::test]
async fn rev_store() {
    use std::ops::RangeInclusive;

    use futures::stream::{self, Stream, StreamExt};

    use tokio::fs::remove_dir_all;

    use cachalot::cachalot;

    #[cachalot(root = ".tests_rev_store")]
    async fn source1<'a>(
        _key: &'a str,
        range: RangeInclusive<u128>,
    ) -> impl Stream<Item = i128> + 'a {
        stream::iter(range).map(|i| -(i as i128))
    }

    assert_eq!(
        source1("rev", 100..=10000).await.collect::<Vec<_>>().await,
        source1("rev", 100..=10000).await.collect::<Vec<_>>().await
    );

    remove_dir_all(".tests_rev_store").await.unwrap()
}

#[tokio::test]
async fn x2_store() {
    use std::hash::Hash;
    use std::ops::Range;

    use futures::stream::{self, Stream, StreamExt, TryStreamExt};

    use tokio::fs::remove_dir_all;

    use cachalot::try_cachalot;

    #[derive(Debug)]
    pub struct MyError {}

    #[try_cachalot(root = ".tests_x2_store")]
    async fn source2<'a, K>(
        _key: K,
        range: Range<u128>,
    ) -> impl Stream<Item = Result<u128, MyError>> + 'a
    where
        K: Send + Sync + Copy + Hash + 'a + 'static,
    {
        stream::iter(range).map(|i| Ok(i * 2))
    }

    assert_eq!(
        source2("x2", 1600..8000)
            .await
            .try_collect::<Vec<_>>()
            .await
            .unwrap(),
        source2("x2", 1600..8000)
            .await
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
    );

    remove_dir_all(".tests_x2_store").await.unwrap()
}

#[tokio::test]
async fn complex_test() {
    use std::ops::Range;

    use futures::stream::{self, Stream, StreamExt};

    use tokio::fs::remove_dir_all;

    use cachalot::cachalot;

    #[cachalot(root = ".tests_complex_test")]
    async fn source(
        _key: &'static str,
        f: fn(u128) -> u128,
        range: Range<u128>,
    ) -> impl Stream<Item = u128> {
        stream::iter(range).map(f)
    }

    fn x2(i: u128) -> u128 {
        i * 2
    }

    fn mod32(i: u128) -> u128 {
        i % 32
    }

    assert_eq!(
        source("x2", x2, 1600..8000).await.collect::<Vec<_>>().await,
        source("x2", x2, 1600..8000).await.collect::<Vec<_>>().await,
    );
    assert_eq!(
        source("mod 32", mod32, 1600..8000)
            .await
            .collect::<Vec<_>>()
            .await,
        source("mod 32", mod32, 1600..8000)
            .await
            .collect::<Vec<_>>()
            .await,
    );

    remove_dir_all(".tests_complex_test").await.unwrap()
}
