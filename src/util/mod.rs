#[macro_use]
mod macros;

pub mod wfc;

#[allow(invalid_reference_casting)]
pub fn as_mut<T>(v: &T) -> &mut T {
    unsafe { &mut *((v as *const T) as *mut T) }
}

pub trait AsMutRef<T> {
    fn as_mut(&self) -> &mut T;
}

impl AsMutRef<crate::voxel::Chunk> for Arc<crate::voxel::Chunk> {
    fn as_mut(&self) -> &mut crate::voxel::Chunk {
        as_mut(self.as_ref())
    }
}

impl AsMutRef<crate::voxel::Vox> for crate::voxel::Vox {
    fn as_mut(&self) -> &mut crate::voxel::Vox {
        as_mut(self)
    }
}

// impl<T, U> AsRefMut<U> for T where T: AsRef<U> {
//     fn as_ref_mut(&self) -> &mut U {
//         as_mut(self.as_ref())
//     }
// }

pub mod registry;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub mod iter {
    use bevy::math::IVec3;

    // [min to max] yzx order
    pub fn iter_aabb(nxz: i32, ny: i32, mut func: impl FnMut(IVec3)) {
        for ly in -ny..=ny {
            for lz in -nxz..=nxz {
                for lx in -nxz..=nxz {
                    func(IVec3::new(lx, ly, lz));
                }
            }
        }
    }

    pub fn iter_center_spread(nxz: i32, ny: i32, mut func: impl FnMut(IVec3)) {
        let max_n = nxz.max(ny);
        for n in 0..=max_n {
            for y in -n..=n {
                for z in -n..=n {
                    for x in -n..=n {
                        if x.abs() < n && y.abs() < n && z.abs() < n {
                            continue;
                        }
                        if x.abs() > nxz || y.abs() > ny || z.abs() > nxz {
                            continue;
                        }
                        func(IVec3::new(x, y, z));
                    }
                }
            }
        }
    }

    pub fn iter_xzy(n: i32, mut func: impl FnMut(IVec3)) {
        assert!(n > 0);
        for ly in 0..n {
            for lz in 0..n {
                for lx in 0..n {
                    func(IVec3::new(lx, ly, lz));
                }
            }
        }
    }
}

use bevy::prelude::*;
use serde::de::DeserializeOwned;

pub fn hash(i: i32) -> f32 {
    let i = (i << 13) ^ i;
    // (((i * i * 15731 + 789221) * i + 1376312589) as u32 & 0xffffffffu32) as f32 / 0xffffffffu32 as f32
    // wrapping_mul: avoid overflow
    let i = i
        .wrapping_mul(i)
        .wrapping_mul(15731)
        .wrapping_add(789221)
        .wrapping_mul(i)
        .wrapping_add(1376312589);
    i as u32 as f32 / 0xffffffffu32 as f32
}
pub fn hash3(v: IVec3) -> Vec3 {
    Vec3::new(hash(v.x), hash(v.y), hash(v.z))
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
        ((t / u).floor() - ((t - dt) / u).floor()) as usize
    }
}
impl TimeIntervals for bevy::time::Time {
    fn intervals(&self, u: f32) -> usize {
        Self::_intervals(self.elapsed_seconds(), self.delta_seconds(), u)
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn hashcode<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[derive(Default)]
pub struct SmoothValue {
    pub target: f32,
    pub current: f32,
}

impl SmoothValue {
    pub fn update(&mut self, dt: f32) {
        self.current += dt * (self.target - self.current);
    }
}

pub mod raw {
    pub unsafe fn as_ref<'a, T>(ptr: *const T) -> &'a T {
        &*ptr
    }

    pub unsafe fn as_mut<'a, T>(ptr: *mut T) -> &'a mut T {
        &mut *ptr
    }
}

pub fn generate_simple_user_name() -> String {
    static ADJS: [&str; 5] = ["Happy", "Sunny", "Sweet", "Bright", "Cheerful"];
    static NOUNS: [&str; 10] = [
        "Apple", "Banana", "Orange", "Mango", "Grapes", "Cherry", "Lime", "Peach", "Pear", "Steven",
    ];

    use rand::Rng;

    let mut rng = rand::thread_rng();
    format!(
        "{}{}{}",
        ADJS[rng.gen_range(0..ADJS.len())],
        NOUNS[rng.gen_range(0..NOUNS.len())],
        rng.gen_range(5..9999)
    )
}

pub fn http_get_json<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    let client = reqwest::blocking::Client::builder().build()?;
    Ok(serde_json::from_value(client.get(url).send()?.json()?)?)
}

pub async fn http_get_json_async<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    // let client = reqwest::Client::builder().build()?;
    // Ok(serde_json::from_value(client.get(url).send().await?.json().await?)?)
    Ok(serde_json::from_value(reqwest::get(url).await?.json().await?)?)
}

// pub fn get_server_list(url: &str) -> anyhow::Result<Vec<crate::game_client::ServerListItem>> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         Err(anyhow::anyhow!("Not supported at this time"))
//     }
//     #[cfg(not(target_arch = "wasm32"))]
//     {
//         http_get_json(url)
//     }
// }
