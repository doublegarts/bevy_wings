//! Paging.

/// A page request. Pages are zero based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    /// Page index.
    pub page: usize,
    /// Items per page (at least 1).
    pub size: usize,
}

impl PageRequest {
    /// Page `page` with `size` items.
    pub fn new(page: usize, size: usize) -> Self {
        Self {
            page,
            size: size.max(1),
        }
    }

    /// Index of the first item.
    pub fn offset(&self) -> usize {
        self.page * self.size
    }

    /// The following page.
    pub fn next(&self) -> Self {
        Self::new(self.page + 1, self.size)
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::new(0, 20)
    }
}

/// One page of results.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    /// Items of this page.
    pub items: Vec<T>,
    /// The request that produced it.
    pub request: PageRequest,
    /// Total number of items across all pages.
    pub total: usize,
}

impl<T> Page<T> {
    /// Cuts a page out of a complete list.
    pub fn from_slice(all: &[T], request: PageRequest) -> Self
    where
        T: Clone,
    {
        let start = request.offset().min(all.len());
        let end = (start + request.size).min(all.len());
        Self {
            items: all[start..end].to_vec(),
            request,
            total: all.len(),
        }
    }

    /// True when more pages follow.
    pub fn has_next(&self) -> bool {
        self.request.offset() + self.items.len() < self.total
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.total.div_ceil(self.request.size)
    }

    /// Maps the items.
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            request: self.request,
            total: self.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slicing, counting and the last page.
    #[test]
    fn paging() {
        let all: Vec<u32> = (0..45).collect();
        let p = Page::from_slice(&all, PageRequest::new(0, 20));
        assert_eq!(p.items.len(), 20);
        assert!(p.has_next());
        assert_eq!(p.page_count(), 3);
        let last = Page::from_slice(&all, PageRequest::new(2, 20));
        assert_eq!(last.items, (40..45).collect::<Vec<_>>());
        assert!(!last.has_next());
        let beyond = Page::from_slice(&all, PageRequest::new(9, 20));
        assert!(beyond.items.is_empty());
        assert_eq!(PageRequest::new(0, 0).size, 1);
    }
}
