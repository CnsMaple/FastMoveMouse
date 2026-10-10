//! Label allocation and progressive prefix filtering.
//!
//! Elements are labelled with letters so the user can narrow the selection down
//! by typing one character at a time. With 26 letters a single character names at
//! most 26 elements, so longer labels are used when there are more elements:
//!
//! * `<= 26`   elements -> `A`..`Z`          (1 char)
//! * `<= 676`  elements -> `AA`..`ZZ`        (2 chars)
//! * `<= 17576` elements -> `AAA`..`ZZZ`     (3 chars)

/// Letters used for labelling.
pub const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Maximum number of elements that can be addressed (26^3).
pub const MAX_LABELS: usize = 26 * 26 * 26;

/// Error returned when there are more elements than labels can address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyElements(pub usize);

impl std::fmt::Display for TooManyElements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "too many elements to label: {} (maximum is {})",
            self.0, MAX_LABELS
        )
    }
}

impl std::error::Error for TooManyElements {}

/// Number of characters needed to label `count` elements.
///
/// Returns `None` when `count` exceeds `MAX_LABELS`.
pub fn label_width(count: usize) -> Option<usize> {
    if count > MAX_LABELS {
        return None;
    }
    if count <= 26 {
        Some(1)
    } else if count <= 26 * 26 {
        Some(2)
    } else {
        Some(3)
    }
}

/// Convert a zero-based index into a fixed-width base-26 label.
///
/// The most significant letter comes first, so labels sort in index order and
/// share prefixes: index 0 -> "A", 25 -> "Z", 26 -> "BA".
///
/// `width` must be in `1..=3`.
pub fn index_to_label(index: usize, width: usize) -> String {
    debug_assert!((1..=3).contains(&width), "width out of range: {width}");
    let mut buf = [0u8; 3];
    let mut n = index;
    // Fill from the least significant position backwards.
    for pos in (0..width).rev() {
        buf[pos] = ALPHABET.as_bytes()[n % 26];
        n /= 26;
    }
    String::from_utf8_lossy(&buf[..width]).into_owned()
}

/// Assign labels to `count` elements in order.
///
/// Returns an empty vector for `count == 0`, or `Err` when there are too many.
pub fn allocate_labels(count: usize) -> Result<Vec<String>, TooManyElements> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let width = label_width(count).ok_or(TooManyElements(count))?;
    Ok((0..count).map(|i| index_to_label(i, width)).collect())
}

/// An element carrying its assigned label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labelled<T> {
    pub label: String,
    pub value: T,
}

impl<T> Labelled<T> {
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// Narrow `items` to those whose label starts with `prefix`.
///
/// Matching is case-insensitive and the consumed leading character is stripped
/// from the retained labels, so repeated calls keep narrowing the selection.
pub fn filter_by_prefix<T: Clone>(items: &[Labelled<T>], prefix: char) -> Vec<Labelled<T>> {
    let want = prefix.to_ascii_uppercase();
    items
        .iter()
        .filter(|it| it.label.chars().next().is_some_and(|c| c == want))
        .map(|it| Labelled {
            label: it.label[1..].to_string(),
            value: it.value.clone(),
        })
        .collect()
}

/// Re-label `items` so their labels are dense again after filtering.
///
/// Filtering leaves gaps (picking "A" out of "AA"/"AB" leaves "A"/"B" which is
/// already dense, but picking a middle branch does not), so the survivors are
/// re-numbered to keep the next key press predictable.
pub fn relabel<T: Clone>(items: &[Labelled<T>]) -> Vec<Labelled<T>> {
    let Ok(labels) = allocate_labels(items.len()) else {
        return items.to_vec();
    };
    items
        .iter()
        .zip(labels)
        .map(|(it, label)| Labelled {
            label,
            value: it.value.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_is_one_up_to_26() {
        assert_eq!(label_width(1), Some(1));
        assert_eq!(label_width(26), Some(1));
        assert_eq!(label_width(27), Some(2));
        assert_eq!(label_width(676), Some(2));
        assert_eq!(label_width(677), Some(3));
        assert_eq!(label_width(MAX_LABELS), Some(3));
        assert_eq!(label_width(MAX_LABELS + 1), None);
    }

    #[test]
    fn single_char_labels_are_a_to_z() {
        let labels = allocate_labels(26).unwrap();
        assert_eq!(labels[0], "A");
        assert_eq!(labels[25], "Z");
    }

    #[test]
    fn two_char_labels_are_aa_to_zz() {
        let labels = allocate_labels(676).unwrap();
        assert_eq!(labels[0], "AA");
        assert_eq!(labels[1], "AB");
        assert_eq!(labels[25], "AZ");
        assert_eq!(labels[26], "BA");
        assert_eq!(labels[675], "ZZ");
    }

    /// Regression test for the C++ bug where the 3rd character reused `i % 26`
    /// and the 2nd character was therefore never advanced: labels collided and
    /// elements were silently dropped when stored in a keyed map.
    #[test]
    fn three_char_labels_are_unique_across_full_range() {
        let labels = allocate_labels(MAX_LABELS).unwrap();
        let mut seen = std::collections::HashSet::new();
        for l in &labels {
            assert!(seen.insert(l.clone()), "duplicate label produced: {l}");
        }
        assert_eq!(seen.len(), MAX_LABELS);
    }

    #[test]
    fn three_char_labels_advance_each_position() {
        let labels = allocate_labels(MAX_LABELS).unwrap();
        assert_eq!(labels[0], "AAA");
        assert_eq!(labels[1], "AAB");
        assert_eq!(labels[25], "AAZ");
        assert_eq!(labels[26], "ABA");
        assert_eq!(labels[675], "AZZ");
        assert_eq!(labels[676], "BAA");
        assert_eq!(labels[MAX_LABELS - 1], "ZZZ");
    }

    #[test]
    fn labels_sort_in_index_order() {
        let labels = allocate_labels(1000).unwrap();
        let mut sorted = labels.clone();
        sorted.sort();
        assert_eq!(labels, sorted);
    }

    #[test]
    fn zero_elements_yield_no_labels() {
        assert_eq!(allocate_labels(0).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn too_many_elements_is_an_error() {
        assert_eq!(
            allocate_labels(MAX_LABELS + 1),
            Err(TooManyElements(MAX_LABELS + 1))
        );
    }

    #[test]
    fn filter_consumes_leading_char_and_is_case_insensitive() {
        let items = vec![
            Labelled::new("AA", 1),
            Labelled::new("AB", 2),
            Labelled::new("BA", 3),
        ];
        let got = filter_by_prefix(&items, 'a');
        assert_eq!(got, vec![Labelled::new("A", 1), Labelled::new("B", 2)]);
    }

    #[test]
    fn repeated_filtering_narrows_to_one() {
        let labels = allocate_labels(5).unwrap();
        let mut items: Vec<_> = labels
            .into_iter()
            .zip(0..5)
            .map(|(l, v)| Labelled::new(l, v))
            .collect();
        items = filter_by_prefix(&items, 'C');
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, 2);
        assert_eq!(items[0].label, "");
    }

    #[test]
    fn filtering_an_unknown_prefix_yields_nothing() {
        let items = vec![Labelled::new("A", 1), Labelled::new("B", 2)];
        assert!(filter_by_prefix(&items, 'z').is_empty());
    }

    #[test]
    fn relabel_makes_labels_dense() {
        let items = vec![Labelled::new("AB", 1), Labelled::new("AC", 2)];
        let got = relabel(&items);
        assert_eq!(got, vec![Labelled::new("A", 1), Labelled::new("B", 2)]);
    }
}
