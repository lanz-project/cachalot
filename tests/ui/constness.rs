use cachalot_proc_macro::cachalot;

#[cachalot]
const fn source(
    key: &'static str,
    range: std::ops::RangeInclusive<usize>,
) -> futures::stream::BoxStream<'static, usize> {
    Box::pin(futures::stream::iter([1usize]))
}

fn main() {}
