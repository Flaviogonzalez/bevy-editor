//! The open documents besides the one being edited, in the order of their tabs.
//!
//! The edited document stays in `Editor::doc`, where all the editing code reaches it, and sits at
//! [`Tabs::active`] in the tab order; the others wait here. Switching tabs swaps one in and the
//! edited one out, so each keeps its own graph, undo history, selection and camera.
//!
//! Input: documents opened, switched to and closed by the editor. Output: which document each tab
//! holds. No GPUI, so it's tested here.

use std::path::Path;

use super::Document;

#[derive(Default)]
pub struct Tabs {
    /// The tabs other than the active one, in order.
    others: Vec<Document>,
    /// Where the edited document's tab sits among all of them.
    active: usize,
}

impl Tabs {
    /// How many tabs there are, the edited document's included.
    pub fn len(&self) -> usize {
        self.others.len() + 1
    }

    pub fn active(&self) -> usize {
        self.active
    }

    /// The document in tab `i`; `current` is the edited one.
    pub fn get<'a>(&'a self, current: &'a Document, i: usize) -> Option<&'a Document> {
        match i.cmp(&self.active) {
            std::cmp::Ordering::Less => self.others.get(i),
            std::cmp::Ordering::Equal => Some(current),
            std::cmp::Ordering::Greater => self.others.get(i - 1),
        }
    }

    /// Every open document, in tab order.
    pub fn all<'a>(&'a self, current: &'a Document) -> impl Iterator<Item = &'a Document> {
        (0..self.len()).filter_map(move |i| self.get(current, i))
    }

    /// The tab of the document saved at `path`, if one is open.
    pub fn find(&self, current: &Document, path: &Path) -> Option<usize> {
        (0..self.len()).find(|&i| self.get(current, i).and_then(|d| d.path.as_deref()) == Some(path))
    }

    /// Makes tab `i` the edited one: `current` goes back to its tab, and tab `i`'s document is
    /// returned to be edited. Returns `current` itself when `i` is already active or doesn't exist.
    pub fn switch(&mut self, current: Document, i: usize) -> Document {
        if i == self.active || i >= self.len() {
            return current;
        }
        let next = self.others.remove(if i < self.active { i } else { i - 1 });
        // With tab `i` out, the edited document's tab goes back where it was among the rest.
        let back = if i < self.active { self.active - 1 } else { self.active };
        self.others.insert(back, current);
        self.active = i;
        next
    }

    /// Opens a tab after the active one, for a document about to be edited: `current` waits in
    /// its tab.
    pub fn open(&mut self, current: Document) {
        self.others.insert(self.active, current);
        self.active += 1;
    }

    /// Closes the edited tab. Returns the document to edit next (the tab after it, or before it
    /// when it was the last), or `None` when it was the only tab.
    pub fn close_active(&mut self) -> Option<Document> {
        if self.others.is_empty() {
            return None;
        }
        if self.active == self.others.len() {
            self.active -= 1;
        }
        Some(self.others.remove(self.active))
    }

    /// Closes tab `i`, which isn't the edited one.
    pub fn close(&mut self, i: usize) {
        if i == self.active || i >= self.len() {
            return;
        }
        if i < self.active {
            self.others.remove(i);
            self.active -= 1;
        } else {
            self.others.remove(i - 1);
        }
    }

    /// The documents waiting in the other tabs.
    pub fn others_mut(&mut self) -> impl Iterator<Item = &mut Document> {
        self.others.iter_mut()
    }

    /// Closes every tab but the edited one.
    pub fn close_others(&mut self) {
        self.others.clear();
        self.active = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn doc(name: &str) -> Document {
        Document { path: Some(PathBuf::from(name)), ..Document::default() }
    }

    fn names(tabs: &Tabs, current: &Document) -> Vec<String> {
        tabs.all(current).map(|d| d.path.as_ref().unwrap().display().to_string()).collect()
    }

    #[test]
    fn tabs_keep_their_order_while_switching() {
        let mut tabs = Tabs::default();
        let mut current = doc("a");
        for name in ["b", "c"] {
            tabs.open(std::mem::replace(&mut current, doc(name)));
        }
        assert_eq!(names(&tabs, &current), ["a", "b", "c"]);
        assert_eq!(tabs.active(), 2);

        current = tabs.switch(current, 0);
        assert_eq!((names(&tabs, &current), tabs.active()), (vec!["a".into(), "b".into(), "c".into()], 0));
        assert_eq!(current.path, Some(PathBuf::from("a")));
        current = tabs.switch(current, 1);
        assert_eq!(current.path, Some(PathBuf::from("b")));
        assert_eq!(names(&tabs, &current), ["a", "b", "c"]);
        assert_eq!(tabs.find(&current, Path::new("c")), Some(2));
        assert_eq!(tabs.find(&current, Path::new("b")), Some(1));
    }

    #[test]
    fn a_new_tab_opens_after_the_active_one() {
        let mut tabs = Tabs::default();
        let mut current = doc("a");
        tabs.open(std::mem::replace(&mut current, doc("b")));
        current = tabs.switch(current, 0);
        tabs.open(std::mem::replace(&mut current, doc("new")));
        assert_eq!(names(&tabs, &current), ["a", "new", "b"]);
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn closing_moves_to_a_neighbour() {
        let mut tabs = Tabs::default();
        let mut current = doc("a");
        for name in ["b", "c"] {
            tabs.open(std::mem::replace(&mut current, doc(name)));
        }
        current = tabs.switch(current, 1);
        // The tab after it takes its place.
        current = tabs.close_active().unwrap();
        assert_eq!((names(&tabs, &current), tabs.active()), (vec!["a".into(), "c".into()], 1));
        // The last tab closes towards the one before.
        current = tabs.close_active().unwrap();
        assert_eq!(names(&tabs, &current), ["a"]);
        assert!(tabs.close_active().is_none());

        tabs.open(std::mem::replace(&mut current, doc("b")));
        tabs.close(0);
        assert_eq!((names(&tabs, &current), tabs.active()), (vec!["b".into()], 0));
    }
}
