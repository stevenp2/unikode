use cursive::Vec2;

use super::buffer::Buffer;

#[derive(Clone)]
struct RowChange {
    index: usize,
    before: Option<Vec<char>>,
    after: Option<Vec<char>>,
}

/// A reversible change to committed canvas rows and cursor state.
#[derive(Clone)]
pub(crate) struct Transaction {
    rows: Vec<RowChange>,
    before_len: usize,
    after_len: usize,
    cursor_before: Option<Vec2>,
    cursor_after: Option<Vec2>,
}

impl Transaction {
    pub(crate) fn between(before: &Buffer, after: &Buffer) -> Option<Self> {
        let row_count = before.chars.len().max(after.chars.len());
        let rows = (0..row_count)
            .filter_map(|index| {
                let old = before.chars.get(index);
                let new = after.chars.get(index);
                (old != new).then(|| RowChange {
                    index,
                    before: old.cloned(),
                    after: new.cloned(),
                })
            })
            .collect::<Vec<_>>();

        if rows.is_empty() {
            return None;
        }

        Some(Self {
            rows,
            before_len: before.chars.len(),
            after_len: after.chars.len(),
            cursor_before: before.cursor,
            cursor_after: after.cursor,
        })
    }

    pub(crate) fn undo(&self, buffer: &mut Buffer) {
        self.apply(buffer, self.before_len, self.cursor_before, |change| {
            change.before.as_ref()
        });
    }

    pub(crate) fn redo(&self, buffer: &mut Buffer) {
        self.apply(buffer, self.after_len, self.cursor_after, |change| {
            change.after.as_ref()
        });
    }

    fn apply<'a, F>(&'a self, buffer: &mut Buffer, row_count: usize, cursor: Option<Vec2>, row: F)
    where
        F: Fn(&'a RowChange) -> Option<&'a Vec<char>>,
    {
        buffer.edits.clear();
        buffer.chars.resize_with(row_count, Vec::new);
        for change in &self.rows {
            if change.index < row_count {
                buffer.chars[change.index] = row(change).cloned().unwrap_or_default();
            }
        }
        buffer.cursor = cursor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_restores_inserted_and_removed_ragged_rows() {
        let before = Buffer::read_from("a\nbb\n".as_bytes()).unwrap();
        let after = Buffer::read_from("bb\nccc\n".as_bytes()).unwrap();
        let transaction = Transaction::between(&before, &after).expect("rows changed");

        let mut canvas = after.clone();
        transaction.undo(&mut canvas);
        assert_eq!(canvas.chars, before.chars);
        transaction.redo(&mut canvas);
        assert_eq!(canvas.chars, after.chars);
    }

    #[test]
    fn cursor_only_changes_do_not_create_history_entries() {
        let before = Buffer::default();
        let mut after = before.clone();
        after.cursor = Some(Vec2::new(2, 3));
        assert!(Transaction::between(&before, &after).is_none());
    }
}
