use crate::entity::EntityID;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

pub struct DataEntry<T> {
    pub data: T,
    pub entity_id: EntityID,
}

pub struct ComponentArray<T> {
    data: Vec<DataEntry<T>>,
    map: HashMap<EntityID, usize>,
}

impl<T> ComponentArray<T> {
    pub fn new() -> Self {
        ComponentArray::<T> {
            data: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn push(&mut self, entity_id: EntityID, data: T) -> &mut T {
        debug_assert!(!self.map.contains_key(&entity_id));
        self.map.insert(entity_id, self.data.len());
        self.data.push(DataEntry::<T> { data, entity_id });
        &mut self.data.last_mut().unwrap().data
    }

    pub fn remove(&mut self, entity_id: EntityID) -> T {
        debug_assert!(self.map.contains_key(&entity_id));
        let index = self.map[&entity_id];
        self.map.remove(&entity_id);
        for entry in &mut self.map {
            if *entry.1 > index {
                *entry.1 -= 1;
            }
        }
        self.data.remove(index).data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn contains_entity(&self, entity_id: EntityID) -> bool {
        self.map.contains_key(&entity_id)
    }
}

impl<T> Index<EntityID> for ComponentArray<T> {
    type Output = DataEntry<T>;

    fn index(&self, entity_id: EntityID) -> &Self::Output {
        debug_assert!(self.map.contains_key(&entity_id));
        let index = self.map[&entity_id];
        &self.data[index]
    }
}

impl<T> IndexMut<EntityID> for ComponentArray<T> {
    fn index_mut(&mut self, entity_id: EntityID) -> &mut Self::Output {
        debug_assert!(self.map.contains_key(&entity_id));
        let index = self.map[&entity_id];
        &mut self.data[index]
    }
}

impl<'a, T> IntoIterator for &'a ComponentArray<T> {
    type Item = &'a DataEntry<T>;
    type IntoIter = std::slice::Iter<'a, DataEntry<T>>;

    fn into_iter(self) -> std::slice::Iter<'a, DataEntry<T>> {
        self.data.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ComponentArray<T> {
    type Item = &'a mut DataEntry<T>;
    type IntoIter = std::slice::IterMut<'a, DataEntry<T>>;

    fn into_iter(self) -> std::slice::IterMut<'a, DataEntry<T>> {
        self.data.iter_mut()
    }
}
