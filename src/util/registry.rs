use bevy::platform::collections::HashMap;

pub type RegId = u16;

#[derive(Default, bevy::prelude::Resource)]
pub struct Registry {
    pub vec: Vec<String>,

    map: HashMap<String, RegId>,
}

impl Registry {
    pub fn insert(&mut self, str_id: &str) -> RegId {
        let num_id = self.vec.len() as RegId;

        self.vec.push(str_id.into());
        self.map.insert(str_id.into(), num_id);
        num_id
    }

    pub fn at(&self, num_id: RegId) -> Option<&String> {
        self.vec.get(num_id as usize)
    }

    pub fn get(&self, str_id: &String) -> Option<RegId> {
        let num_id = *self.map.get(str_id)?;
        Some(num_id)
    }

    pub fn build_num_id(&mut self) {
        self.vec.sort_unstable();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}
