use bevy::utils::HashMap;

pub type RegId = u16;

#[derive(Default)]
pub struct Registry<T> {
    vec: Vec<(String, T)>,

    map: HashMap<String, RegId>,
}

impl<T> Registry<T> {
    pub fn insert(&mut self, str_id: String, val: T) {
        let num_id = self.vec.len() as RegId;

        self.vec.push((str_id.clone(), val));
        self.map.insert(str_id, num_id);
    }

    pub fn at(&self, num_id: RegId) -> Option<&(String, T)> {
        /*
        if let Some(e) = self.vec.get(num_id as usize) {
            Some(e)
        } else {
            None
        }
        */
        self.vec.get(num_id as usize)
    }

    pub fn get(&self, str_id: &String) -> Option<(RegId, &T)> {
        /*
        if let Some(num_id) = self.map.get(str_id) {
            if let Some(e) = self.at(*num_id) {
                Some((*num_id, &e.1))
            } else {
                None
            }
        } else {
            None
        }
        */
        let num_id = self.map.get(str_id)?;
        Some((*num_id, &self.at(*num_id)?.1))
    }

    pub fn sort_id(&mut self) {
        self.vec.sort_unstable_by(|a, b| (a.0).cmp(&b.0));
    }
}
