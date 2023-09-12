use futures::{Future, Stream};

use bytemuck::{AnyBitPattern, NoUninit};

use crate::IdxRange;

pub trait SourceRange: From<IdxRange> + TryInto<IdxRange, Error = Self> {}

impl<Range> SourceRange for Range where Range: From<IdxRange> + TryInto<IdxRange, Error = Range> {}

pub trait Source<R, K, V>: Fn(K, R) -> Self::Fut + Sync
where
    V: NoUninit + AnyBitPattern,
{
    type Stream: Stream<Item = V> + Send;
    type Fut: Future<Output = Self::Stream> + Send;
}

impl<F, R, K, V, St, Fut> Source<R, K, V> for F
where
    F: Fn(K, R) -> Fut + Sync,
    V: NoUninit + AnyBitPattern,
    St: Stream<Item = V> + Send,
    Fut: Future<Output = St> + Send,
{
    type Stream = St;
    type Fut = Fut;
}

pub trait TrySource<R, K, V>: Fn(K, R) -> Self::Fut + Sync
where
    V: NoUninit + AnyBitPattern,
{
    type Error: Send + Sync + 'static;
    type Stream: Stream<Item = Result<V, Self::Error>> + Send;
    type Fut: Future<Output = Self::Stream> + Send;
}

impl<F, R, K, V, E, St, Fut> TrySource<R, K, V> for F
where
    F: Fn(K, R) -> Fut + Sync,
    V: NoUninit + AnyBitPattern,
    E: Send + Sync + 'static,
    St: Stream<Item = Result<V, E>> + Send,
    Fut: Future<Output = St> + Send,
{
    type Error = E;
    type Stream = St;
    type Fut = Fut;
}
