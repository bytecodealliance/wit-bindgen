use std::cell::{Cell, RefCell};
use std::convert::TryFrom;
use std::mem;
use std::ops::Deref;

pub struct Table<T> {
    elems: RefCell<Vec<Slot<T>>>,
    next: Cell<usize>,
}

pub struct Borrow<'a, T> {
    table: &'a Table<T>,
    ptr: &'a T,
    index: usize,
}

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
            elems: Default::default(),
            next: Cell::new(0),
        }
    }

    /// Inserts an item into this table, returning the index that it was
    /// inserted at.
    pub fn insert(&self, item: T) -> u32 {
        let mut elems = self.elems.borrow_mut();
        if self.next.get() == elems.len() {
            elems.push(Slot::Empty {
                next_empty: self.next.get() + 1,
            });
        }
        let index = self.next.get();
        let ret = u32::try_from(index).unwrap();
        self.next.set(match &elems[index] {
            Slot::Empty { next_empty } => *next_empty,
            Slot::Full { .. } => unreachable!(),
        });
        elems[index] = Slot::Full {
            item: Box::new(item),
            uses: 0,
        };
        return ret;
    }

    /// Borrows an item from this table.
    ///
    /// Returns `None` if the index is not allocated at this time. Otherwise
    /// returns `Some` with a borrow of the item from this table.
    ///
    /// While an item is borrowed from this table it cannot be removed.
    pub fn get(&self, item: u32) -> Option<Borrow<'_, T>> {
        let mut elems = self.elems.borrow_mut();
        let index = usize::try_from(item).unwrap();
        match elems.get_mut(index) {
            Some(Slot::Empty { .. }) | None => None,
            Some(Slot::Full { item, uses }) => {
                *uses += 1;
                let item = &**item as *const T;
                Some(Borrow {
                    table: self,
                    ptr: unsafe { &*item },
                    index,
                })
            }
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
        let mut elems = self.elems.borrow_mut();
        let index = usize::try_from(item).unwrap();
        let new_empty = Slot::Empty {
            next_empty: self.next.get(),
        };
        let slot = elems.get_mut(index).ok_or(RemoveError::NotAllocated)?;

        // Assume that `item` is valid, and if it is, we can return quickly
        match mem::replace(slot, new_empty) {
            Slot::Full { uses: 0, item } => {
                self.next.set(index);
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

impl<T> Deref for Borrow<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.ptr
    }
}

impl<T> Drop for Borrow<'_, T> {
    fn drop(&mut self) {
        let mut elems = self.table.elems.borrow_mut();
        match &mut elems[self.index] {
            Slot::Full { uses, .. } => *uses -= 1,
            Slot::Empty { .. } => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let table = Table::new();
        assert_eq!(table.insert(0), 0);
        assert_eq!(table.insert(100), 1);
        assert_eq!(table.insert(200), 2);

        assert_eq!(*table.get(0).unwrap(), 0);
        assert_eq!(*table.get(1).unwrap(), 100);
        assert_eq!(*table.get(2).unwrap(), 200);
        assert!(table.get(100).is_none());

        assert!(table.remove(0).is_ok());
        assert!(table.get(0).is_none());
        assert_eq!(table.insert(1), 0);
        assert!(table.get(0).is_some());

        let borrow = table.get(1).unwrap();
        assert!(table.remove(1).is_err());
        drop(borrow);
        assert!(table.remove(1).is_ok());
        assert!(table.remove(1).is_err());

        assert!(table.remove(2).is_ok());
        assert!(table.remove(0).is_ok());

        assert_eq!(table.insert(100), 0);
        assert_eq!(table.insert(100), 2);
        assert_eq!(table.insert(100), 1);
        assert_eq!(table.insert(100), 3);

        let borrow1 = table.get(3).unwrap();
        let borrow2 = table.get(3).unwrap();
        assert!(table.remove(3).is_err());
        drop(borrow1);
        assert!(table.remove(3).is_err());
        drop(borrow2);
        assert!(table.remove(3).is_ok());
    }
}
