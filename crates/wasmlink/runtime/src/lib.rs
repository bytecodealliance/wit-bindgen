use std::mem;

pub struct Slab<T> {
    storage: Vec<Entry<T>>,
    next: usize,
}

enum Entry<T> {
    Full(T),
    Empty { next: usize },
}

impl<T> Slab<T> {
    pub fn insert(&mut self, item: T) -> u32 {
        if self.next == self.storage.len() {
            self.storage.push(Entry::Empty {
                next: self.next + 1,
            });
        }
        let ret = self.next as u32;
        let entry = Entry::Full(item);
        self.next = match mem::replace(&mut self.storage[self.next], entry) {
            Entry::Empty { next } => next,
            _ => unreachable!(),
        };
        return ret;
    }

    pub fn get(&self, idx: u32) -> Option<&T> {
        match self.storage.get(idx as usize)? {
            Entry::Full(b) => Some(b),
            Entry::Empty { .. } => None,
        }
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<&mut T> {
        match self.storage.get_mut(idx as usize)? {
            Entry::Full(b) => Some(b),
            Entry::Empty { .. } => None,
        }
    }

    pub fn remove(&mut self, idx: u32) -> Option<T> {
        let slot = self.storage.get_mut(idx as usize)?;
        match mem::replace(slot, Entry::Empty { next: self.next }) {
            Entry::Full(b) => {
                self.next = idx as usize;
                Some(b)
            }
            Entry::Empty { next } => {
                *slot = Entry::Empty { next };
                None
            }
        }
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Slab<T> {
        Slab {
            storage: Vec::new(),
            next: 0,
        }
    }
}

#[derive(Default)]
pub struct IndexSlab {
    slab: Slab<ResourceIndex>,
}

impl IndexSlab {
    pub fn insert(&mut self, resource: ResourceIndex) -> u32 {
        self.slab.insert(resource)
    }

    pub fn get(&self, slab_idx: u32) -> Option<ResourceIndex> {
        self.slab.get(slab_idx).cloned()
    }

    pub fn remove(&mut self, slab_idx: u32) -> Option<ResourceIndex> {
        self.slab.remove(slab_idx)
    }
}

#[derive(Default)]
pub struct ResourceSlab {
    slab: Slab<Resource>,
}

#[derive(Debug)]
struct Resource {
    wasm: u32,
    references: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct ResourceIndex(u32);

impl ResourceSlab {
    pub fn insert(&mut self, wasm: u32) -> ResourceIndex {
        ResourceIndex(self.slab.insert(Resource {
            wasm,
            references: 1,
        }))
    }

    pub fn get(&self, idx: ResourceIndex) -> u32 {
        self.slab.get(idx.0).unwrap().wasm
    }

    pub fn clone(&mut self, idx: ResourceIndex) {
        let resource = self.slab.get_mut(idx.0).unwrap();
        resource.references = resource.references.checked_add(1).unwrap();
    }

    pub fn drop(&mut self, idx: ResourceIndex) -> Option<u32> {
        let resource = self.slab.get_mut(idx.0).unwrap();
        assert!(resource.references > 0);
        resource.references -= 1;
        if resource.references != 0 {
            return None;
        }
        let resource = self.slab.remove(idx.0).unwrap();
        Some(resource.wasm)
    }
}

// Currently we assume the runtime cannot be used from multiple threads
// because we don't support Wasm threading.
// In the future this may need to be properly synchronized.
static mut SLABS: Vec<(IndexSlab, ResourceSlab)> = Vec::new();

// This exists to indicate to users that an invalid handle value was passed
// Thus the problem comes from user code and *not* the runtime
fn invalid_handle_trap() -> ! {
    unreachable!()
}

#[no_mangle]
pub extern "C" fn resource_insert(id: usize, res: u32) -> u32 {
    let (indexes, resources) = unsafe {
        if SLABS.len() <= id {
            SLABS.resize_with(id + 1, Default::default);
        }

        SLABS.get_mut(id).unwrap()
    };

    let index = resources.insert(res);
    indexes.insert(index)
}

#[no_mangle]
pub extern "C" fn resource_get(id: usize, idx: u32) -> u32 {
    let (indexes, resources) = unsafe {
        if SLABS.len() <= id {
            SLABS.resize_with(id + 1, Default::default);
        }

        SLABS.get(id).unwrap()
    };

    let res_idx = indexes
        .get(idx)
        .ok_or_else(|| invalid_handle_trap())
        .unwrap();
    resources.get(res_idx)
}

#[no_mangle]
pub extern "C" fn resource_clone(id: usize, idx: u32) -> u32 {
    let (indexes, resources) = unsafe {
        if SLABS.len() <= id {
            SLABS.resize_with(id + 1, Default::default);
        }

        SLABS.get_mut(id).unwrap()
    };

    let res_idx = indexes
        .get(idx)
        .ok_or_else(|| invalid_handle_trap())
        .unwrap();
    resources.clone(res_idx);
    indexes.insert(res_idx)
}

#[no_mangle]
pub extern "C" fn resource_remove(id: usize, idx: u32) -> u64 {
    let (indexes, resources) = unsafe {
        if SLABS.len() <= id {
            SLABS.resize_with(id + 1, Default::default);
        }

        SLABS.get_mut(id).unwrap()
    };

    let res_idx = indexes
        .remove(idx)
        .ok_or_else(|| invalid_handle_trap())
        .unwrap();

    // The return value's upper 32-bits is a flag to denote if the resource is still alive.
    // If the upper 32-bits are 0, the lower 32-bits are expected to be the resource to drop.
    match resources.drop(res_idx) {
        Some(wasm) => wasm as u64,
        None => 1u64 << 32,
    }
}
