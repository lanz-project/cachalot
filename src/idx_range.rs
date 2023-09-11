use std::ops::{Range, RangeInclusive};

use crate::pages::{PageRange, PagesIter};
use crate::Idx;

#[derive(Clone, Debug, PartialEq)]
pub enum IdxRange {
    One(Idx),
    Many(Idx, Idx),
}

impl IdxRange {
    pub fn new(start: Idx, len: usize) -> Option<Self> {
        match len {
            0 => None,
            1 => Some(IdxRange::One(start)),
            _ => Some(IdxRange::Many(start, start + len as Idx - 1)),
        }
    }

    pub fn len(&self) -> Idx {
        match self {
            IdxRange::One(_) => 1,
            IdxRange::Many(start, end) => end - start + 1,
        }
    }

    pub fn pages<const PAGE_SIZE: Idx>(&self) -> impl Iterator<Item = PageRange<PAGE_SIZE>> {
        PagesIter::from_idx_range(self)
    }
}

impl<I> TryFrom<Range<I>> for IdxRange
where
    I: Into<Idx> + Clone,
{
    type Error = Range<I>;

    fn try_from(range: Range<I>) -> Result<Self, Range<I>> {
        let start_index = range.start.clone().into();
        let len = range
            .end
            .clone()
            .into()
            .checked_sub(start_index)
            .unwrap_or(0) as usize;

        IdxRange::new(start_index, len).ok_or(range)
    }
}

impl<I> From<IdxRange> for Range<I>
where
    I: From<Idx> + Default,
{
    fn from(range: IdxRange) -> Range<I> {
        match range {
            IdxRange::One(idx) => idx.into()..(idx + 1).into(),
            IdxRange::Many(from, to) => from.into()..(to + 1).into(),
        }
    }
}

impl<I> TryFrom<RangeInclusive<I>> for IdxRange
where
    I: Into<Idx> + Clone,
{
    type Error = RangeInclusive<I>;

    fn try_from(range: RangeInclusive<I>) -> Result<Self, Self::Error> {
        let start_index = range.start().clone().into();
        let len = (range.end().clone().into() + 1)
            .checked_sub(start_index)
            .unwrap_or(0) as usize;

        IdxRange::new(start_index, len).ok_or(range)
    }
}

impl<I> From<IdxRange> for RangeInclusive<I>
where
    I: From<Idx>,
{
    fn from(range: IdxRange) -> Self {
        match range {
            IdxRange::One(idx) => idx.into()..=idx.into(),
            IdxRange::Many(from, to) => from.into()..=to.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let zero = IdxRange::try_from(10..0 as Idx);
        assert!(zero.is_err());

        let r = 10..11 as Idx;
        let one = IdxRange::try_from(r.clone()).unwrap();
        assert_eq!(one, IdxRange::One(10));
        let one: Range<Idx> = one.into();
        assert_eq!(one, r);

        let r = 10..21 as Idx;
        let many = IdxRange::try_from(r.clone()).unwrap();
        assert_eq!(many, IdxRange::Many(10, 20));
        let many: Range<Idx> = many.into();
        assert_eq!(many, r);
    }

    #[test]
    fn test_range_inclusive() {
        let zero = IdxRange::try_from(10..=0 as Idx);
        assert!(zero.is_err());

        let r = 10..=10 as Idx;
        let one = IdxRange::try_from(r.clone()).unwrap();
        assert_eq!(one, IdxRange::One(10));
        let one: RangeInclusive<Idx> = one.into();
        assert_eq!(one, r);

        let r = 10..=20 as Idx;
        let many = IdxRange::try_from(r.clone()).unwrap();
        assert_eq!(many, IdxRange::Many(10, 20));
        let many: RangeInclusive<Idx> = many.into();
        assert_eq!(many, r);
    }
}
