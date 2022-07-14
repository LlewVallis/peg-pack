use std::hint::unreachable_unchecked;

pub struct ParseResult {
    kind: ParseResultKind,
    scan_distance: usize,
}

impl ParseResult {
    pub fn unmatched(scan_distance: usize) -> Self {
        Self {
            scan_distance,
            kind: ParseResultKind::Unmatched,
        }
    }

    pub fn empty(scan_distance: usize) -> Self {
        Self::error_free(0, scan_distance)
    }

    pub fn error_free(distance: usize, scan_distance: usize) -> Self {
        Self {
            scan_distance,
            kind: ParseResultKind::Matched(Match::error_free(distance)),
        }
    }

    pub unsafe fn combine_matches(first: Self, second: Self) -> Self {
        let scan_distance = Self::combined_scan_distance(&first, &second);

        let first_distance = first.unwrap_distance_unchecked();
        let second_distance = second.unwrap_distance_unchecked();
        let distance = first_distance + second_distance;

        let first_error_distance = first.unwrap_error_distance_unchecked();
        let second_error_distance = second.unwrap_error_distance_unchecked();

        let error_distance = first_error_distance
            .or_else(|| second_error_distance.map(|distance| first_distance + distance));

        let value = Match {
            distance,
            error_distance,
            children: vec![first, second],
        };

        Self {
            kind: ParseResultKind::Matched(value),
            scan_distance,
        }
    }

    pub fn combined_scan_distance(first: &Self, second: &Self) -> usize {
        usize::max(first.scan_distance, first.distance() + second.scan_distance)
    }

    pub fn scan_distance(&self) -> usize {
        self.scan_distance
    }

    pub fn distance(&self) -> usize {
        match &self.kind {
            ParseResultKind::Matched(value) => value.distance,
            ParseResultKind::Unmatched => 0,
        }
    }

    pub unsafe fn unwrap_distance_unchecked(&self) -> usize {
        match &self.kind {
            ParseResultKind::Matched(value) => value.distance,
            ParseResultKind::Unmatched => unreachable_unchecked(),
        }
    }

    pub unsafe fn unwrap_error_distance_unchecked(&self) -> Option<usize> {
        match &self.kind {
            ParseResultKind::Matched(value) => value.error_distance,
            ParseResultKind::Unmatched => unreachable_unchecked(),
        }
    }

    pub fn match_length(&self) -> Option<usize> {
        match &self.kind {
            ParseResultKind::Matched(value) => Some(value.distance),
            ParseResultKind::Unmatched => None,
        }
    }

    pub fn has_matched(&self) -> bool {
        match &self.kind {
            ParseResultKind::Matched(_) => true,
            ParseResultKind::Unmatched => false,
        }
    }

    pub fn is_error_free(&self) -> bool {
        match &self.kind {
            ParseResultKind::Matched(value) => value.error_distance.is_none(),
            ParseResultKind::Unmatched => false,
        }
    }

    pub fn max_scan_distance(mut self, scan_distance: usize) -> Self {
        self.scan_distance = self.scan_distance.max(scan_distance);
        self
    }

    pub fn negate(mut self) -> Self {
        match &mut self.kind {
            ParseResultKind::Matched(_) => {
                self.kind = ParseResultKind::Matched(Match::error_free(0))
            }
            ParseResultKind::Unmatched => self.kind = ParseResultKind::Matched(Match::empty()),
        }

        self
    }

    pub fn mark_error(mut self) -> Self {
        if let ParseResultKind::Matched(value) = &mut self.kind {
            value.error_distance = Some(0);
        }

        self
    }
}

enum ParseResultKind {
    Matched(Match),
    Unmatched,
}

struct Match {
    distance: usize,
    error_distance: Option<usize>,
    children: Vec<ParseResult>,
}

impl Match {
    pub fn empty() -> Self {
        Self::error_free(0)
    }

    pub fn error_free(distance: usize) -> Self {
        Self {
            distance,
            error_distance: None,
            children: Vec::new(),
        }
    }
}
