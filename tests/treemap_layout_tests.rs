use proptest::prelude::*;
use treetop::treemap::algorithm::squarify_sorted;
use treetop::treemap::node::{LayoutRect, TreemapItem};

fn make_items(values: &[u64]) -> Vec<TreemapItem> {
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| TreemapItem {
            pid: i as u32,
            value: v,
            label: format!("p{}", i),
        })
        .collect()
}

fn squarify_for_tests(
    items: &[TreemapItem],
    bounds: &LayoutRect,
) -> Vec<treetop::treemap::node::TreemapRect> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.value.cmp(&a.value));
    squarify_sorted(&sorted, bounds)
}

proptest! {
    #[test]
    fn area_conservation(
        values in prop::collection::vec(1u64..100_000, 1..100),
    ) {
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let items = make_items(&values);
        let rects = squarify_for_tests(&items, &bounds);
        let total_area: f64 = rects.iter().map(|r| r.rect.area()).sum();
        let bounds_area = 120.0 * 40.0;
        prop_assert!(
            (total_area - bounds_area).abs() < 1.0,
            "Area mismatch: {} vs {}", total_area, bounds_area
        );
    }

    #[test]
    fn containment(
        values in prop::collection::vec(1u64..100_000, 1..100),
    ) {
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let items = make_items(&values);
        let rects = squarify_for_tests(&items, &bounds);
        let eps = 0.01;
        for r in &rects {
            prop_assert!(r.rect.x >= -eps, "x out of bounds: {}", r.rect.x);
            prop_assert!(r.rect.y >= -eps, "y out of bounds: {}", r.rect.y);
            prop_assert!(
                r.rect.x + r.rect.width <= 120.0 + eps,
                "x+w out of bounds: {}", r.rect.x + r.rect.width
            );
            prop_assert!(
                r.rect.y + r.rect.height <= 40.0 + eps,
                "y+h out of bounds: {}", r.rect.y + r.rect.height
            );
        }
    }

    #[test]
    fn no_degenerate_rects(
        values in prop::collection::vec(1u64..100_000, 1..100),
    ) {
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let items = make_items(&values);
        let rects = squarify_for_tests(&items, &bounds);
        for r in &rects {
            prop_assert!(r.rect.width > 0.0, "Zero width for id={}", r.pid);
            prop_assert!(r.rect.height > 0.0, "Zero height for id={}", r.pid);
        }
    }

    #[test]
    fn correct_count(
        values in prop::collection::vec(1u64..100_000, 1..50),
    ) {
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let items = make_items(&values);
        let rects = squarify_for_tests(&items, &bounds);
        prop_assert_eq!(rects.len(), items.len());
    }
}
