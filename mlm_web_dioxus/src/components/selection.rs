use std::collections::BTreeSet;

use dioxus::prelude::*;

pub fn update_row_selection<T: Ord + Clone + 'static>(
    event: &MouseEvent,
    mut selected: Signal<BTreeSet<T>>,
    mut last_selected_idx: Signal<Option<usize>>,
    all_row_ids: &[T],
    row_id: &T,
    row_index: usize,
) {
    let will_select = !selected.read().contains(row_id);
    let mut next = selected.read().clone();

    if event.modifiers().shift() {
        if let Some(last_idx) = *last_selected_idx.read() {
            let (start, end) = if last_idx <= row_index {
                (last_idx, row_index)
            } else {
                (row_index, last_idx)
            };
            for id in &all_row_ids[start..=end] {
                if will_select {
                    next.insert(id.clone());
                } else {
                    next.remove(id);
                }
            }
        } else if will_select {
            next.insert(row_id.clone());
        } else {
            next.remove(row_id);
        }
    } else if will_select {
        next.insert(row_id.clone());
    } else {
        next.remove(row_id);
    }

    selected.set(next);
    last_selected_idx.set(Some(row_index));
}
