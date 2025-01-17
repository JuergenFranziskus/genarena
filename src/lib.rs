use std::{iter::FusedIterator, mem::ManuallyDrop, ops::{Index, IndexMut}};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Idx {
    index: u32,
    generation: u32,
}


pub struct IntoIter<T> {
    slots: std::vec::IntoIter<Slot<T>>,
    free_remaining: u32,
}
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for mut slot in &mut self.slots {
            if slot.generation >= 0 {
                unsafe { ManuallyDrop::drop(&mut slot.entry.item) }
            }
        }
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {   

            let slot = self.slots.next_back()?;
            if slot.generation >= 0 {
                break Some(ManuallyDrop::into_inner(unsafe { slot.entry.item }))
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }
}
impl<T> ExactSizeIterator for IntoIter<T> {

}
impl<T> FusedIterator for IntoIter<T> {

}
impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {   
            let slot = self.slots.next()?;
            if slot.generation >= 0 {
                break Some(ManuallyDrop::into_inner(unsafe { slot.entry.item }))
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.slots.len()) - (self.free_remaining as usize);
        (len, Some(len))
    }
}


pub struct Iter<'a, T> {
    slots: std::slice::Iter<'a, Slot<T>>,
    free_remaining: u32,
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {   

            let slot = self.slots.next_back()?;
            if slot.generation >= 0 {
                break Some(unsafe { &slot.entry.item })
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }
}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    
}
impl<'a, T> FusedIterator for Iter<'a, T> {
    
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {   
            let slot = self.slots.next()?;
            if slot.generation >= 0 {
                break Some(unsafe { &slot.entry.item })
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.slots.len()) - (self.free_remaining as usize);
        (len, Some(len))
    }
}

pub struct IterMut<'a, T> {
    slots: std::slice::IterMut<'a, Slot<T>>,
    free_remaining: u32,
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {   
            let slot = self.slots.next_back()?;
            if slot.generation >= 0 {
                break Some(unsafe { &mut slot.entry.item })
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }
}
impl<'a, T> ExactSizeIterator for IterMut<'a, T> {

}
impl<'a, T> FusedIterator for IterMut<'a, T> {

}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {   

            let slot = self.slots.next()?;
            if slot.generation >= 0 {
                break Some(unsafe { &mut slot.entry.item })
            }
            else {
                self.free_remaining -= 1;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.slots.len()) - (self.free_remaining as usize);
        (len, Some(len))
    }
}

pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free_count: u32,
    first_free: u32,
}
impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free_count: 0,
            first_free: u32::MAX,
        }
    }

    pub fn exists(&mut self, idx: Idx) -> bool {
        let i = idx.index as usize;
        if i >= self.slots.len() { return false };
        let slot = &self.slots[i];
        slot.generation == idx.generation as i32
    }
    pub fn get(&self, idx: Idx) -> Option<&T> {
        let i = idx.index as usize;
        if i < self.slots.len() { return None };
        let slot = &self.slots[i];
        if slot.generation != idx.generation as i32 { return None };
        Some(unsafe { &slot.entry.item })
    }
    pub fn get_mut(&mut self, idx: Idx) -> Option<&mut T> {
        let i = idx.index as usize;
        if i < self.slots.len() { return None };
        let slot = &mut self.slots[i];
        if slot.generation != idx.generation as i32 { return None };
        Some(unsafe { &mut slot.entry.item })
    }
    pub fn insert(&mut self, item: T) -> Idx {
        if self.free_count == 0 {
            let index = self.slots.len() as u32;
            self.slots.push(Slot { entry: Entry { item: ManuallyDrop::new(item)}, generation: 0});
            Idx { index, generation: 0 }
        }
        else {
            let index = self.first_free;
            let slot = &mut self.slots[index as usize];
            let generation = -slot.generation;
            slot.generation = generation;
            self.first_free = unsafe { slot.entry.next_free };
            slot.entry.item = ManuallyDrop::new(item);
            self.free_count -= 1;

            Idx { index, generation: generation as u32 }
        }
    }
    pub fn iter(&self) -> Iter<T> {
        Iter {
            slots: self.slots.iter(),
            free_remaining: self.free_count,
        }
    }
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            slots: self.slots.iter_mut(),
            free_remaining: self.free_count,
        }
    }
    pub fn len(&self) -> usize {
        self.slots.len() - self.free_count as usize
    }
    pub fn remove(&mut self, idx: Idx) -> Option<T> {
        let i = idx.index as usize;
        if i >= self.slots.len() { return None };
        let slot = &mut self.slots[i];
        let generation = idx.generation as i32;
        if generation != slot.generation { return None };
        let item = unsafe { ManuallyDrop::take(&mut slot.entry.item) };

        if generation == i32::MAX {
            slot.generation = i32::MIN;
        }
        else {
            let next_generation = generation + 1;
            slot.generation = -next_generation;
            slot.entry.next_free = self.first_free;
            self.first_free = idx.index;
        }

        Some(item)
    }
}
impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            if slot.generation >= 0 {
                unsafe { ManuallyDrop::drop(&mut slot.entry.item); }
            }
        }
    }
}
impl<T> Index<Idx> for Arena<T> {
    type Output = T;
    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<T> IndexMut<Idx> for Arena<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
impl<T> IntoIterator for Arena<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        let slots = std::mem::take(&mut self.slots);
        

        IntoIter {
            slots: slots.into_iter(),
            free_remaining: self.free_count,
        }
    }
}
impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

struct Slot<T> {
    entry: Entry<T>,
    generation: i32,
}
union Entry<T> {
    item: ManuallyDrop<T>,
    next_free: u32,
}
