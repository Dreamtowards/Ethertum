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


pub trait TimeIntervals {
    fn intervals(&self, interval: f32) -> usize;

    fn at_interval(&self, interval: f32) -> bool {
        self.intervals(interval) != 0
    }

    fn _intervals(t: f32, dt: f32, u: f32) -> usize {
        ((t / u).floor() - ((t-dt) / u).floor()) as usize
    }
}
impl TimeIntervals for bevy::time::Time {
    fn intervals(&self, u: f32) -> usize {
        Self::_intervals(self.elapsed_seconds(), self.delta_seconds(), u)
    }
}