//! The row and byte caps, owned once for both engines.
//!
//! Truncation honesty lives here: a page that stops early says so and names the cap it hit.
//! Both wire modules offer rows one at a time and stop reading the moment the accumulator
//! reports itself full, so what travels stays near what is returned.

use protocol::sql::TruncationCause;

/// One rendered row: every cell the engine's text rendering, `None` an SQL `NULL`.
pub type RenderedRow = Vec<Option<String>>;

/// What [`RowAccumulator::offer`] decided about one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// The row was taken; keep reading.
    Taken,
    /// The row was not taken and reading must stop. The page is truncated for the named cause.
    Full(TruncationCause),
}

/// Accumulates rows under a row cap and a byte cap.
///
/// The byte count is the summed UTF-8 length of the rendered cells (`NULL` counts zero) — it
/// bounds what actually travels onward, not the engine's wire framing. A row that would push
/// the total past the byte cap is not taken, even as the first row: the caps are published
/// bounds, and returning a page that exceeds one would make the flag a lie.
#[derive(Debug)]
pub struct RowAccumulator {
    max_rows: u32,
    max_bytes: u64,
    rows: Vec<RenderedRow>,
    bytes: u64,
    truncation: Option<TruncationCause>,
}

impl RowAccumulator {
    /// A fresh accumulator under the given caps.
    #[must_use]
    pub fn new(max_rows: u32, max_bytes: u64) -> Self {
        Self {
            max_rows,
            max_bytes,
            rows: Vec::new(),
            bytes: 0,
            truncation: None,
        }
    }

    /// Offer one row. On [`Offer::Full`] the caller stops reading; offering after that is a
    /// caller bug and keeps refusing.
    pub fn offer(&mut self, row: RenderedRow) -> Offer {
        if let Some(cause) = self.truncation {
            return Offer::Full(cause);
        }
        if self.rows.len() as u32 >= self.max_rows {
            self.truncation = Some(TruncationCause::RowCap);
            return Offer::Full(TruncationCause::RowCap);
        }
        let row_bytes: u64 = row
            .iter()
            .map(|cell| cell.as_deref().map_or(0, str::len) as u64)
            .sum();
        if self.bytes + row_bytes > self.max_bytes {
            self.truncation = Some(TruncationCause::ByteCap);
            return Offer::Full(TruncationCause::ByteCap);
        }
        self.bytes += row_bytes;
        self.rows.push(row);
        Offer::Taken
    }

    /// The accumulated page: rows, counted bytes, and the truncation cause when a cap was hit.
    #[must_use]
    pub fn finish(self) -> (Vec<RenderedRow>, u64, Option<TruncationCause>) {
        (self.rows, self.bytes, self.truncation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> RenderedRow {
        vec![Some(text.to_owned())]
    }

    #[test]
    fn a_result_inside_both_caps_is_not_truncated() {
        let mut accumulator = RowAccumulator::new(3, 1_000);
        assert_eq!(accumulator.offer(row("a")), Offer::Taken);
        assert_eq!(accumulator.offer(row("bb")), Offer::Taken);
        let (rows, bytes, truncation) = accumulator.finish();
        assert_eq!(rows.len(), 2);
        assert_eq!(bytes, 3);
        assert_eq!(truncation, None);
    }

    /// **Row-cap honesty**: exactly `max_rows` rows fit; the row after that flips the flag and
    /// is not returned.
    #[test]
    fn the_row_after_the_cap_truncates_and_is_not_returned() {
        let mut accumulator = RowAccumulator::new(2, 1_000);
        assert_eq!(accumulator.offer(row("a")), Offer::Taken);
        assert_eq!(accumulator.offer(row("b")), Offer::Taken);
        assert_eq!(
            accumulator.offer(row("c")),
            Offer::Full(TruncationCause::RowCap)
        );
        let (rows, _, truncation) = accumulator.finish();
        assert_eq!(rows.len(), 2);
        assert_eq!(truncation, Some(TruncationCause::RowCap));
    }

    /// A result of exactly `max_rows` rows with nothing after it is **not** truncated — the flag
    /// reports the result set, not the cap's own size.
    #[test]
    fn an_exact_fit_is_not_truncated() {
        let mut accumulator = RowAccumulator::new(2, 1_000);
        assert_eq!(accumulator.offer(row("a")), Offer::Taken);
        assert_eq!(accumulator.offer(row("b")), Offer::Taken);
        let (rows, _, truncation) = accumulator.finish();
        assert_eq!(rows.len(), 2);
        assert_eq!(truncation, None);
    }

    /// **Byte-cap honesty**: the row that would cross the byte cap is not taken, and the cause
    /// names the byte cap, not the row cap.
    #[test]
    fn the_row_crossing_the_byte_cap_truncates() {
        let mut accumulator = RowAccumulator::new(100, 10);
        assert_eq!(accumulator.offer(row("aaaa")), Offer::Taken);
        assert_eq!(
            accumulator.offer(row("bbbbbbbb")),
            Offer::Full(TruncationCause::ByteCap)
        );
        let (rows, bytes, truncation) = accumulator.finish();
        assert_eq!(rows.len(), 1);
        assert_eq!(bytes, 4);
        assert_eq!(truncation, Some(TruncationCause::ByteCap));
    }

    /// Even the first row is refused when it alone exceeds the byte cap: an empty, honestly
    /// truncated page beats an oversized one.
    #[test]
    fn a_first_row_over_the_byte_cap_yields_an_empty_truncated_page() {
        let mut accumulator = RowAccumulator::new(100, 4);
        assert_eq!(
            accumulator.offer(row("aaaaaaaa")),
            Offer::Full(TruncationCause::ByteCap)
        );
        let (rows, bytes, truncation) = accumulator.finish();
        assert!(rows.is_empty());
        assert_eq!(bytes, 0);
        assert_eq!(truncation, Some(TruncationCause::ByteCap));
    }

    #[test]
    fn null_cells_count_zero_bytes() {
        let mut accumulator = RowAccumulator::new(10, 2);
        assert_eq!(accumulator.offer(vec![None, None, None]), Offer::Taken);
        let (_, bytes, truncation) = accumulator.finish();
        assert_eq!(bytes, 0);
        assert_eq!(truncation, None);
    }
}
