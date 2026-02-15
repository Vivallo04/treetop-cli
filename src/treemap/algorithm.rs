use super::node::{LayoutRect, TreemapItem, TreemapRect};

pub fn squarify_sorted(items: &[TreemapItem], bounds: &LayoutRect) -> Vec<TreemapRect> {
    if items.is_empty() || bounds.area() <= 0.0 {
        return Vec::new();
    }

    let sorted: Vec<&TreemapItem> = items.iter().collect();
    squarify_sorted_refs(&sorted, bounds)
}

fn squarify_sorted_refs(sorted: &[&TreemapItem], bounds: &LayoutRect) -> Vec<TreemapRect> {
    let total_value: f64 = sorted.iter().map(|i| i.value as f64).sum();
    if total_value <= 0.0 {
        return Vec::new();
    }

    let total_area = bounds.area();
    let mut results = Vec::with_capacity(sorted.len());
    let mut remaining = bounds.clone();

    let mut row: Vec<&TreemapItem> = Vec::new();
    let mut row_area = 0.0;

    for item in sorted {
        let item_area = (item.value as f64 / total_value) * total_area;

        if row.is_empty() {
            row.push(item);
            row_area = item_area;
            continue;
        }

        let side = remaining.shorter_side();
        let worst_without = worst_aspect_ratio(&row, row_area, side);

        row.push(item);
        let new_row_area = row_area + item_area;
        let worst_with = worst_aspect_ratio(&row, new_row_area, side);

        if worst_with <= worst_without {
            row_area = new_row_area;
        } else {
            row.pop();
            layout_row(&row, row_area, &mut remaining, &mut results);
            row.clear();
            row.push(item);
            row_area = item_area;
        }
    }

    if !row.is_empty() {
        layout_row(&row, row_area, &mut remaining, &mut results);
    }

    results
}

fn worst_aspect_ratio(row: &[&TreemapItem], row_area: f64, side: f64) -> f64 {
    if side <= 0.0 || row_area <= 0.0 {
        return f64::MAX;
    }

    let row_value_sum: f64 = row.iter().map(|i| i.value as f64).sum();
    if row_value_sum <= 0.0 {
        return f64::MAX;
    }

    let mut worst = 0.0_f64;
    for item in row {
        let frac = item.value as f64 / row_value_sum;
        let item_area = frac * row_area;
        if item_area <= 0.0 {
            continue;
        }
        let strip_thickness = row_area / side;
        let item_length = item_area / strip_thickness;
        let aspect = if strip_thickness > item_length {
            strip_thickness / item_length
        } else {
            item_length / strip_thickness
        };
        worst = worst.max(aspect);
    }
    worst
}

fn layout_row(
    row: &[&TreemapItem],
    row_area: f64,
    remaining: &mut LayoutRect,
    results: &mut Vec<TreemapRect>,
) {
    if row.is_empty() || remaining.area() <= 0.0 {
        return;
    }

    let row_value_sum: f64 = row.iter().map(|i| i.value as f64).sum();
    if row_value_sum <= 0.0 {
        return;
    }

    let vertical = remaining.width >= remaining.height;

    if vertical {
        let strip_width = row_area / remaining.height;
        let mut y = remaining.y;

        for item in row {
            let frac = item.value as f64 / row_value_sum;
            let item_height = frac * remaining.height;

            results.push(TreemapRect {
                rect: LayoutRect::new(remaining.x, y, strip_width, item_height),
                pid: item.pid,
                label: item.label.clone(),
                value: item.value,
            });

            y += item_height;
        }

        remaining.x += strip_width;
        remaining.width -= strip_width;
    } else {
        let strip_height = row_area / remaining.width;
        let mut x = remaining.x;

        for item in row {
            let frac = item.value as f64 / row_value_sum;
            let item_width = frac * remaining.width;

            results.push(TreemapRect {
                rect: LayoutRect::new(x, remaining.y, item_width, strip_height),
                pid: item.pid,
                label: item.label.clone(),
                value: item.value,
            });

            x += item_width;
        }

        remaining.y += strip_height;
        remaining.height -= strip_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squarify_for_tests(items: &[TreemapItem], bounds: &LayoutRect) -> Vec<TreemapRect> {
        let mut sorted = items.to_vec();
        sorted.sort_by(|a, b| b.value.cmp(&a.value));
        squarify_sorted(&sorted, bounds)
    }

    #[test]
    fn empty_input() {
        let rects = squarify_for_tests(&[], &LayoutRect::new(0.0, 0.0, 100.0, 100.0));
        assert!(rects.is_empty());
    }

    #[test]
    fn single_item() {
        let items = vec![TreemapItem {
            pid: 1,
            label: "A".into(),
            value: 100,
        }];
        let bounds = LayoutRect::new(0.0, 0.0, 80.0, 40.0);
        let rects = squarify_for_tests(&items, &bounds);
        assert_eq!(rects.len(), 1);
        let r = &rects[0];
        assert!((r.rect.width * r.rect.height - 3200.0).abs() < 1.0);
    }

    #[test]
    fn two_equal_items() {
        let items = vec![
            TreemapItem {
                pid: 1,
                label: "A".into(),
                value: 50,
            },
            TreemapItem {
                pid: 2,
                label: "B".into(),
                value: 50,
            },
        ];
        let bounds = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        let rects = squarify_for_tests(&items, &bounds);
        assert_eq!(rects.len(), 2);
        let total_area: f64 = rects.iter().map(|r| r.rect.area()).sum();
        assert!((total_area - 10000.0).abs() < 1.0);
        // Each should have roughly half the area
        for r in &rects {
            assert!((r.rect.area() - 5000.0).abs() < 1.0);
        }
    }

    #[test]
    fn area_conservation() {
        let items: Vec<TreemapItem> = (0..20)
            .map(|i| TreemapItem {
                pid: i,
                label: format!("p{i}"),
                value: (i as u64 + 1) * 100,
            })
            .collect();
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let rects = squarify_for_tests(&items, &bounds);
        let total_area: f64 = rects.iter().map(|r| r.rect.area()).sum();
        assert!(
            (total_area - bounds.area()).abs() < 1.0,
            "Area mismatch: {total_area} vs {}",
            bounds.area()
        );
    }

    #[test]
    fn containment() {
        let items: Vec<TreemapItem> = (0..30)
            .map(|i| TreemapItem {
                pid: i,
                label: format!("p{i}"),
                value: (i as u64 + 1) * 50,
            })
            .collect();
        let bounds = LayoutRect::new(0.0, 0.0, 120.0, 40.0);
        let rects = squarify_for_tests(&items, &bounds);
        let eps = 0.01;
        for r in &rects {
            assert!(r.rect.x >= bounds.x - eps);
            assert!(r.rect.y >= bounds.y - eps);
            assert!(r.rect.x + r.rect.width <= bounds.x + bounds.width + eps);
            assert!(r.rect.y + r.rect.height <= bounds.y + bounds.height + eps);
        }
    }
}
