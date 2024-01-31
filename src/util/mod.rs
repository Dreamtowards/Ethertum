#[macro_use]
mod macros;
pub use macros::hashmap;

// pub mod registry;

use std::time::{SystemTime, Duration};


pub mod iter {
    use bevy::math::IVec3;


    pub fn iter_aabb(nxz: i32, ny: i32, mut func: impl FnMut(&IVec3)) {
        for ly in -ny..=ny {
            for lz in -nxz..=nxz {
                for lx in -nxz..=nxz {
                    func(&IVec3::new(lx,ly,lz));
                }
            }
        }
    }

}


pub fn current_timestamp() -> Duration {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
}

pub fn current_timestamp_millis() -> u64 {
    current_timestamp().as_millis() as u64
}