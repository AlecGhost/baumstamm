/// Minimum-cost, order-preserving assignment of continuous horizontal targets
/// to collision-free integer grid columns.
pub(super) fn project_to_slots(targets: &[f64], row_length: usize) -> Vec<usize> {
    if targets.is_empty() {
        return Vec::new();
    }
    assert!(
        targets.len() <= row_length,
        "layer cannot fit in its grid row"
    );

    let count = targets.len();
    let mut costs = vec![vec![f64::INFINITY; row_length]; count];
    let mut predecessors = vec![vec![usize::MAX; row_length]; count];
    for (slot, cost) in costs[0].iter_mut().enumerate() {
        *cost = (slot as f64 - targets[0]).powi(2);
    }
    for person in 1..count {
        let mut best = (f64::INFINITY, usize::MAX);
        for slot in 0..row_length {
            if slot > 0 && costs[person - 1][slot - 1] < best.0 {
                best = (costs[person - 1][slot - 1], slot - 1);
            }
            if best.1 != usize::MAX {
                costs[person][slot] = best.0 + (slot as f64 - targets[person]).powi(2);
                predecessors[person][slot] = best.1;
            }
        }
    }

    let mut slot = (0..row_length)
        .min_by(|first, second| costs[count - 1][*first].total_cmp(&costs[count - 1][*second]))
        .expect("non-empty row");
    let mut result = vec![0; count];
    for person in (0..count).rev() {
        result[person] = slot;
        if person > 0 {
            slot = predecessors[person][slot];
        }
    }
    result
}
