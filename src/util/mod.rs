pub mod registry;



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