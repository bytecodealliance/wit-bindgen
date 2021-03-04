use std::cell::RefCell;
use std::convert::TryFrom;
use std::fmt;
use std::mem;

pub struct Table<T> {
    inner: RefCell<Inner<T>>,
}

struct Inner<T> {
    elems: Vec<Slot<T>>,
    next: usize,
    active_borrows: Vec<usize>,
    access_idx: usize,
}

pub struct Borrows<'a, T> {
    table: &'a Table<T>,
    start: usize,
    access_idx: usize,
}

#[derive(Debug)]
pub enum RemoveError {
    NotAllocated,
    InUse,
}

enum Slot<T> {
    Empty { next_empty: usize },
    Full { item: Box<T>, uses: u32 },
}

impl<T> Table<T> {
    /// Creates a new empty table
    pub fn new() -> Table<T> {
        Table {
            inner: RefCell::new(Inner {
                elems: Vec::new(),
                next: 0,
                active_borrows: Vec::new(),
                access_idx: 0,
            }),
        }
    }

    /// Inserts an item into this table, returning the index that it was
    /// inserted at.
    pub fn insert(&self, item: T) -> u32 {
        let mut inner = self.inner.borrow_mut();
        if inner.next == inner.elems.len() {
            let next_empty = inner.next + 1;
            inner.elems.push(Slot::Empty { next_empty });
        }
        let index = inner.next;
        let ret = u32::try_from(index).unwrap();
        inner.next = match &inner.elems[index] {
            Slot::Empty { next_empty } => *next_empty,
            Slot::Full { .. } => unreachable!(),
        };
        inner.elems[index] = Slot::Full {
            item: Box::new(item),
            uses: 0,
        };
        return ret;
    }

    pub fn access(&self) -> Borrows<'_, T> {
        let mut inner = self.inner.borrow_mut();
        inner.access_idx += 1;
        Borrows {
            table: self,
            access_idx: inner.access_idx,
            start: inner.active_borrows.len(),
        }
    }

    /// Removes an item from this table.
    ///
    /// On success it returns back the original item.
    ///
    /// This can fail for two reasons:
    ///
    /// * First the item specified may not be an allocated index
    /// * Second the item specified may be in active use by a call to `get`
    pub fn remove(&self, item: u32) -> Result<T, RemoveError> {
        let mut inner = self.inner.borrow_mut();
        let index = usize::try_from(item).unwrap();
        let new_empty = Slot::Empty {
            next_empty: inner.next,
        };
        let slot = inner
            .elems
            .get_mut(index)
            .ok_or(RemoveError::NotAllocated)?;

        // Assume that `item` is valid, and if it is, we can return quickly
        match mem::replace(slot, new_empty) {
            Slot::Full { uses: 0, item } => {
                inner.next = index;
                Ok(*item)
            }

            // Oops `item` wasn't valid, put it back where we found it and then
            // figure out why it was invalid
            prev => {
                *slot = prev;
                match *slot {
                    Slot::Empty { .. } => Err(RemoveError::NotAllocated),
                    Slot::Full { .. } => Err(RemoveError::InUse),
                }
            }
        }
    }
}

impl<T> Default for Table<T> {
    fn default() -> Table<T> {
        Table::new()
    }
}

impl<T> Borrows<'_, T> {
    /// Borrows an item from this table.
    ///
    /// Returns `None` if the index is not allocated at this time. Otherwise
    /// returns `Some` with a borrow of the item from this table.
    ///
    /// While an item is borrowed from this table it cannot be removed.
    pub fn get(&self, item: u32) -> Option<&T> {
        let mut inner = self.table.inner.borrow_mut();
        let inner = &mut *inner;
        let index = usize::try_from(item).unwrap();
        match inner.elems.get_mut(index) {
            Some(Slot::Empty { .. }) | None => None,
            Some(Slot::Full { item, uses }) => {
                inner.active_borrows.push(index);
                *uses += 1;
                Some(unsafe { &*(&**item as *const T) })
            }
        }
    }
}

impl<T> Drop for Borrows<'_, T> {
    fn drop(&mut self) {
        let mut inner = self.table.inner.borrow_mut();
        let inner = &mut *inner;
        assert_eq!(
            inner.access_idx, self.access_idx,
            "table was not accessed with `Borrows` in a stack-like fashion"
        );
        inner.access_idx -= 1;
        for index in inner.active_borrows.drain(self.start..) {
            match &mut inner.elems[index] {
                Slot::Full { uses, .. } => *uses -= 1,
                Slot::Empty { .. } => unreachable!(),
            }
        }
    }
}

impl fmt::Display for RemoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoveError::NotAllocated => f.write_str("invalid handle index"),
            RemoveError::InUse => f.write_str("table index in use"),
        }
    }
}

impl std::error::Error for RemoveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let table = Table::new();
        assert_eq!(table.insert(0), 0);
        assert_eq!(table.insert(100), 1);
        assert_eq!(table.insert(200), 2);

        let borrows = table.access();
        assert_eq!(*borrows.get(0).unwrap(), 0);
        assert_eq!(*borrows.get(1).unwrap(), 100);
        assert_eq!(*borrows.get(2).unwrap(), 200);
        assert!(borrows.get(100).is_none());
        drop(borrows);

        assert!(table.remove(0).is_ok());
        assert!(table.access().get(0).is_none());
        assert_eq!(table.insert(1), 0);
        assert!(table.access().get(0).is_some());

        let borrows = table.access();
        borrows.get(1).unwrap();
        assert!(table.remove(1).is_err());
        drop(borrows);
        assert!(table.remove(1).is_ok());
        assert!(table.remove(1).is_err());

        assert!(table.remove(2).is_ok());
        assert!(table.remove(0).is_ok());

        assert_eq!(table.insert(100), 0);
        assert_eq!(table.insert(100), 2);
        assert_eq!(table.insert(100), 1);
        assert_eq!(table.insert(100), 3);

        let borrows1 = table.access();
        borrows1.get(3);
        let borrows2 = table.access();
        borrows2.get(3);
        assert!(table.remove(3).is_err());
        drop(borrows2);
        assert!(table.remove(3).is_err());
        drop(borrows1);
        assert!(table.remove(3).is_ok());
    }
}
