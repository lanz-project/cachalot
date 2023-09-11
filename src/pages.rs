use num::Integer;

use accumulable::Accumulable;

use crate::{Idx, IdxRange};

#[derive(Debug, PartialEq)]
pub struct PageRange<const PAGE_SIZE: Idx> {
    pub(crate) page: Idx,
    pub(crate) first: Idx,
    pub(crate) last: Idx,
}

impl<const PAGE_SIZE: Idx> PageRange<PAGE_SIZE> {
    pub fn new(page: Idx, first: Idx, last: Idx) -> Self {
        Self { page, first, last }
    }

    pub fn full_fill(&self) -> bool {
        self.first == 0 && self.last == (PAGE_SIZE - 1)
    }

    pub fn len(&self) -> usize {
        (self.last + 1 - self.first) as usize
    }
}

#[derive(Clone)]
pub struct PagesRange<const PAGE_SIZE: Idx> {
    pub(crate) from: Idx,
    pub(crate) first: Idx,
    pub(crate) to: Idx,
    pub(crate) last: Idx,
}

impl<const PAGE_SIZE: Idx> PagesRange<PAGE_SIZE> {
    pub fn pages(&self) -> impl Iterator<Item = PageRange<PAGE_SIZE>> {
        PagesIter::from_pages_range(self)
    }

    pub fn len(&self) -> usize {
        (self.to + 1 - self.from) as usize
    }
}

impl<const PAGE_SIZE: Idx> Accumulable for PagesRange<PAGE_SIZE> {
    fn accumulate_from(&mut self, rhs: &Self) {
        if self.to == rhs.from {
            assert_eq!(self.last + 1, rhs.first);
        } else {
            assert_eq!(self.last, PAGE_SIZE - 1);
            assert_eq!(rhs.first, 0);
            assert_eq!(self.to + 1, rhs.from);
        }

        self.to = rhs.to;
        self.last = rhs.last;
    }
}

impl<const PAGE_SIZE: Idx> From<PageRange<PAGE_SIZE>> for PagesRange<PAGE_SIZE> {
    fn from(page: PageRange<PAGE_SIZE>) -> Self {
        Self {
            from: page.page,
            first: page.first,
            to: page.page,
            last: page.last,
        }
    }
}

impl<const PAGE_SIZE: Idx> From<&IdxRange> for PagesRange<PAGE_SIZE> {
    fn from(range: &IdxRange) -> Self {
        match range {
            IdxRange::One(index) => {
                let (page, index) = index.div_rem(&PAGE_SIZE);

                Self {
                    from: page,
                    first: index,
                    to: page,
                    last: index,
                }
            }
            IdxRange::Many(start, end) => {
                let (from, first) = start.div_rem(&PAGE_SIZE);
                let (to, last) = end.div_rem(&PAGE_SIZE);

                Self {
                    from,
                    first,
                    to,
                    last,
                }
            }
        }
    }
}

impl<const PAGE_SIZE: Idx> Into<IdxRange> for PagesRange<PAGE_SIZE> {
    fn into(self) -> IdxRange {
        let start = self.from * PAGE_SIZE + self.first;
        let end = self.to * PAGE_SIZE + self.last;

        IdxRange::new(start, (end + 1 - start) as usize).unwrap()
    }
}

#[cfg(test)]
#[test]
fn test_pages_range_into_idx_range() {
    let range = IdxRange::new(10, 5).unwrap();
    let pages = PagesRange::<2>::from(&range);
    assert_eq!(range, pages.into())
}

#[derive(Debug)]
pub struct PagesIter<const PAGE_SIZE: Idx> {
    first: Idx,
    to: Idx,
    last: Idx,
    left: Idx,
}

impl<const PAGE_SIZE: Idx> PagesIter<PAGE_SIZE> {
    pub fn from_idx_range(range: &IdxRange) -> Self {
        match range {
            IdxRange::One(index) => {
                let (page, index) = index.div_rem(&PAGE_SIZE);

                Self {
                    first: index,
                    to: page,
                    last: index,
                    left: 1,
                }
            }
            IdxRange::Many(start, end) => {
                let (from, first) = start.div_rem(&PAGE_SIZE);
                let (to, last) = end.div_rem(&PAGE_SIZE);

                Self {
                    first,
                    to,
                    last,
                    left: (to + 1) - from,
                }
            }
        }
    }

    pub fn from_pages_range(pages_range: &PagesRange<PAGE_SIZE>) -> Self {
        let left = (pages_range.to + 1) - pages_range.from;

        Self {
            first: pages_range.first,
            to: pages_range.to,
            last: pages_range.last,
            left,
        }
    }
}

impl<const PAGE_SIZE: Idx> Iterator for PagesIter<PAGE_SIZE> {
    type Item = PageRange<PAGE_SIZE>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            None
        } else if self.left == 1 {
            self.left = 0;

            Some(PageRange::new(self.to, self.first, self.last))
        } else {
            self.left = self.left - 1;

            let page = PageRange::new(self.to - self.left, self.first, PAGE_SIZE - 1);

            self.first = 0;

            Some(page)
        }
    }
}

#[cfg(test)]
mod page_iter_tests {
    use super::*;

    #[test]
    fn test_one_page() {
        let r: IdxRange = (10..=10 as Idx).try_into().unwrap();
        let mut pages = r.pages::<5>();
        assert_eq!(pages.next(), Some(PageRange::new(2, 0, 0)));
        assert_eq!(pages.next(), None);

        let r: IdxRange = (16..=17 as Idx).try_into().unwrap();
        let mut pages = r.pages::<3>();
        assert_eq!(pages.next(), Some(PageRange::new(5, 1, 2)));
        assert_eq!(pages.next(), None);
    }

    #[test]
    fn test_two_page() {
        let r: IdxRange = (10..=15 as Idx).try_into().unwrap();
        let mut pages = r.pages::<5>();
        assert_eq!(pages.next(), Some(PageRange::new(2, 0, 4)));
        assert_eq!(pages.next(), Some(PageRange::new(3, 0, 0)));
        assert_eq!(pages.next(), None);

        let r: IdxRange = (17..=18 as Idx).try_into().unwrap();
        let mut pages = r.pages::<3>();
        assert_eq!(pages.next(), Some(PageRange::new(5, 2, 2)));
        assert_eq!(pages.next(), Some(PageRange::new(6, 0, 0)));
        assert_eq!(pages.next(), None);
    }

    #[test]
    fn test_three_page() {
        let r: IdxRange = (14..=22 as Idx).try_into().unwrap();
        let mut pages = r.pages::<5>();
        assert_eq!(pages.next(), Some(PageRange::new(2, 4, 4)));
        assert_eq!(pages.next(), Some(PageRange::new(3, 0, 4)));
        assert_eq!(pages.next(), Some(PageRange::new(4, 0, 2)));
        assert_eq!(pages.next(), None);
    }
}
