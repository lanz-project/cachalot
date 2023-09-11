use futures::{Future, Stream};

use bytemuck::{AnyBitPattern, NoUninit};

use crate::IdxRange;

pub trait SourceRange: From<IdxRange> + TryInto<IdxRange, Error = Self> {}

impl<Range> SourceRange for Range where Range: From<IdxRange> + TryInto<IdxRange, Error = Range> {}

pub trait Source<Range, K, V, S, Fut>: Fn(K, Range) -> Fut + Sync
where
    V: NoUninit + AnyBitPattern,
    S: Stream<Item = V>,
    Fut: Future<Output = S>
{
}

impl<F, Range, K, V, S, Fut> Source<Range, K, V, S, Fut> for F
where
    F: Fn(K, Range) -> Fut + Sync,
    V: NoUninit + AnyBitPattern,
    S: Stream<Item = V>,
    Fut: Future<Output = S>
{
}

pub trait TrySource<Range, K, V, E, S, Fut>: Fn(K, Range) -> Fut + Sync
where
    V: NoUninit + AnyBitPattern,
    S: Stream<Item = Result<V, E>>,
    Fut: Future<Output = S>
{
}

impl<F, Range, K, V, E, S, Fut> TrySource<Range, K, V, E, S, Fut> for F
where
    F: Fn(K, Range) -> Fut + Sync,
    V: NoUninit + AnyBitPattern,
    S: Stream<Item = Result<V, E>>,
    Fut: Future<Output = S>
{
}
