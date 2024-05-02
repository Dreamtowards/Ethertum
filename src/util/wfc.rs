use bevy::{math::vec3, prelude::*};
use rand::Rng;

use crate::util::as_mut;

pub struct SocketId {
    pub shape_id: u16,
    pub is_flip: bool,
}

pub struct Pattern {
    pub name: String,

    pub is_flipped: bool,
    pub rotation: u8,

    pub sockets: [u32; 6],
}
impl Pattern {
    pub fn new(name: String, sockets: [u32; 6], rotation: u8) -> Self {
        Self {
            name,
            is_flipped: false,
            rotation,
            sockets,
        }
    }
}

fn sockets_rotate_y(sockets: [u32; 6]) -> [u32; 6] {
    [sockets[4], sockets[5], sockets[2], sockets[3], sockets[1], sockets[0]]
}

fn sockets_flip_x(sockets: [u32; 6]) -> [u32; 6] {
    [sockets[1], sockets[0], sockets[2], sockets[3], sockets[4], sockets[5]]
}

pub struct Tile {
    pub pos: IVec3,
    pub neighbors: [bool; 6],
    pub possib: Vec<u16>, // superpatterns: Bitset<D>, coefficient
}
impl Tile {
    const DIR: [Vec3; 6] = [
        // 6 Faces
        vec3(-1., 0., 0.),
        vec3(1., 0., 0.),
        vec3(0., -1., 0.),
        vec3(0., 1., 0.),
        vec3(0., 0., -1.),
        vec3(0., 0., 1.),
    ];
    fn opposite_dir_idx(i: usize) -> usize {
        i / 2 * 2 + (i + 1) % 2
    }

    fn new(pos: IVec3, pat: u16) -> Self {
        // for i in possib.iter() {
        // let weight = wfc.patterns[i].frequency;
        // self.sum_weights += weight;
        // self.sum_weights_log_weights += weight * weight.log();
        // }

        Self {
            pos,
            neighbors: [false; 6],
            possib: (0..pat).collect(),
        }
    }

    fn is_collapsed(&self) -> bool {
        self.entropy() == 1
    }

    // num superpatterns, num coefficients
    pub fn entropy(&self) -> usize {
        self.possib.len()
    }

    fn collapse(&mut self) {
        assert!(!self.is_collapsed());

        // // Left one.
        // while self.possib.len() > 1 {
        //     self.possib.pop();
        // }
        let tmp = self.possib[rand::thread_rng().gen_range(0..self.possib.len())];
        self.possib.clear();
        self.possib.push(tmp);
    }

    // fn get_collapsed_pattern() -> &Pattern {

    // }

    // return: true if changed so keep propagate, false is non change so skip propagate.
    fn constrain(&mut self, oppo_possib: &Vec<u16>, dir_idx: usize, all_pat: &Vec<Pattern>) -> bool {
        if self.is_collapsed() {
            return false;
        }

        let mut sockets = Vec::new();
        for possib in oppo_possib {
            sockets.push(all_pat[*possib as usize].sockets[dir_idx]);
        }

        let old_len = self.entropy();

        self.possib
            .retain(|possib| sockets.contains(&all_pat[*possib as usize].sockets[Tile::opposite_dir_idx(dir_idx)]));

        old_len != self.entropy()
    }
}

fn idx_3d_pos_inbound(pos: IVec3, extent: IVec3) -> bool {
    pos.min_element() >= 0 && pos.x < extent.x && pos.y < extent.y && pos.z < extent.z
}
fn idx_3d(pos: IVec3, extent: IVec3) -> usize {
    (pos.x + extent.x * (pos.y + extent.y * pos.z)) as usize
}
fn idx_3d_reveal(idx: i32, extent: IVec3) -> IVec3 {
    let z = idx / (extent.x * extent.y);
    let y = (idx - z * extent.x * extent.y) / extent.x;
    let x = idx - z * extent.x * extent.y - y * extent.x;
    IVec3::new(x, y, z)
}

pub struct WFC {
    pub extent: IVec3,
    pub tiles: Vec<Tile>,

    pub all_patterns: Vec<Pattern>,
}

impl Default for WFC {
    fn default() -> Self {
        Self::new()
    }
}

impl WFC {
    pub fn new() -> Self {
        Self {
            extent: IVec3::ZERO,
            tiles: Vec::new(),
            all_patterns: Vec::new(),
        }
    }

    pub fn push_pattern(&mut self, name: String, sockets: [u32; 6], rot: bool, flip: bool) {
        // let flip = true;

        self.all_patterns.push(Pattern {
            name: name.clone(),
            is_flipped: false,
            rotation: 0,
            sockets,
        });
        if flip {
            self.all_patterns.push(Pattern {
                name: name.clone(),
                is_flipped: true,
                rotation: 0,
                sockets: sockets_flip_x(sockets),
            });
        }

        if rot {
            self.all_patterns.push(Pattern {
                name: name.clone(),
                is_flipped: false,
                rotation: 1,
                sockets: sockets_rotate_y(sockets),
            });
            self.all_patterns.push(Pattern {
                name: name.clone(),
                is_flipped: false,
                rotation: 2,
                sockets: sockets_rotate_y(sockets_rotate_y(sockets)),
            });
            self.all_patterns.push(Pattern {
                name: name.clone(),
                is_flipped: false,
                rotation: 3,
                sockets: sockets_rotate_y(sockets_rotate_y(sockets_rotate_y(sockets))),
            });

            if flip {
                self.all_patterns.push(Pattern {
                    name: name.clone(),
                    is_flipped: true,
                    rotation: 1,
                    sockets: sockets_rotate_y(sockets_flip_x(sockets)),
                });
                self.all_patterns.push(Pattern {
                    name: name.clone(),
                    is_flipped: true,
                    rotation: 2,
                    sockets: sockets_rotate_y(sockets_rotate_y(sockets_flip_x(sockets))),
                });
                self.all_patterns.push(Pattern {
                    name: name.clone(),
                    is_flipped: true,
                    rotation: 3,
                    sockets: sockets_rotate_y(sockets_rotate_y(sockets_rotate_y(sockets_flip_x(sockets)))),
                });
            }
        }
    }

    pub fn init_tiles(&mut self, extent: IVec3) {
        self.extent = extent;

        let num_tiles = (extent.x * extent.y * extent.z) as usize;
        self.tiles.reserve(num_tiles);

        for idx in 0..num_tiles {
            let p = idx_3d_reveal(idx as i32, extent);
            self.tiles.push(Tile::new(p, self.all_patterns.len() as u16));
        }
    }

    pub fn get_tile(&self, pos: IVec3) -> Option<&Tile> {
        if !idx_3d_pos_inbound(pos, self.extent) {
            return None;
        }
        Some(&self.tiles[idx_3d(pos, self.extent)])
    }
    pub fn get_tile_mut(&mut self, pos: IVec3) -> Option<&mut Tile> {
        if !idx_3d_pos_inbound(pos, self.extent) {
            return None;
        }
        Some(&mut self.tiles[idx_3d(pos, self.extent)])
    }

    pub fn run(&mut self) {
        while let Some(tile) = self.next_tile_to_observe() {
            as_mut(tile).collapse();
            info!("Found one to collapse");

            self.propagate(tile);
        }
    }

    // find next tile to collapse/observe. used Minimal-Entropy Heuristic here, due to human sence / predictability / stability
    fn next_tile_to_observe(&self) -> Option<&Tile> {
        let mut ret = None;
        let mut min = usize::MAX;
        for tile in self.tiles.iter() {
            let n = tile.entropy();
            if n > 1 && n < min {
                min = n;
                ret = Some(tile);
            }
        }
        ret
    }

    fn propagate(&self, tile: &Tile) {
        // DFS
        let mut stack = Vec::new();
        stack.push(tile);

        while let Some(tile) = stack.pop() {
            for (dir_idx, neib_dir) in Tile::DIR.iter().enumerate() {
                let neib_pos = tile.pos + neib_dir.as_ivec3();

                if let Some(neib_tile) = self.get_tile(neib_pos) {
                    if neib_tile.is_collapsed() {
                        continue;
                    }

                    if as_mut(neib_tile).constrain(&tile.possib, dir_idx, &self.all_patterns) {
                        // propagate changed value
                        stack.push(neib_tile); // when possibilities reduced need to propagate further.
                    }
                }
            }
        }
    }
}




// #[derive(Component)]
// struct WfcTest;

// fn wfc_test(
//     mut cmds: Commands,
//     asset_server: Res<AssetServer>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut meshes: ResMut<Assets<Mesh>>,

//     mut ctx: bevy_egui::EguiContexts,
//     query_wfc: Query<Entity, With<WfcTest>>,

//     mut tx_templ_name: Local<String>,
// ) {
//     bevy_egui::egui::Window::new("WFC").show(ctx.ctx_mut(), |ui| {
//         ui.text_edit_singleline(&mut *tx_templ_name);

//         if ui.btn("ReGen").clicked() {
//             for e_wfc in query_wfc.iter() {
//                 cmds.entity(e_wfc).despawn_recursive();
//             }

//             use crate::util::wfc::*;
//             let mut wfc = WFC::new();
//             wfc.push_pattern("0".into(), [0; 6], false, false);
//             wfc.push_pattern("1".into(), [1; 6], false, false);
//             wfc.push_pattern("2".into(), [0, 2, 0, 0, 0, 0], true, false);
//             wfc.push_pattern("3".into(), [3, 3, 0, 0, 0, 0], true, false);
//             wfc.push_pattern("4".into(), [1, 2, 0, 0, 4, 4], true, false);
//             wfc.push_pattern("5".into(), [4, 0, 0, 0, 4, 0], true, false);
//             wfc.push_pattern("6".into(), [2, 2, 0, 0, 0, 0], true, false);
//             wfc.push_pattern("7".into(), [2, 2, 0, 0, 3, 3], true, false);
//             wfc.push_pattern("8".into(), [0, 0, 0, 0, 3, 2], true, false);
//             wfc.push_pattern("9".into(), [2, 2, 0, 0, 2, 0], true, false);
//             wfc.push_pattern("10".into(), [2, 2, 0, 0, 2, 2], true, false);
//             wfc.push_pattern("11".into(), [0, 2, 0, 0, 2, 0], true, false);
//             wfc.push_pattern("12".into(), [2, 2, 0, 0, 0, 0], true, false);
//             wfc.init_tiles(IVec3::new(15, 1, 15));

//             wfc.run();

//             for tile in wfc.tiles.iter() {
//                 if tile.entropy() == 0 {
//                     continue; // ERROR
//                 }
//                 let pat = &wfc.all_patterns[tile.possib[0] as usize];

//                 cmds.spawn((
//                     PbrBundle {
//                         mesh: meshes.add(Plane3d::new(Vec3::Y)),
//                         material: materials.add(StandardMaterial {
//                             base_color_texture: Some(asset_server.load(format!("test/comp/circuit{}/{}.png", &*tx_templ_name, pat.name))),
//                             unlit: true,
//                             ..default()
//                         }),
//                         transform: Transform::from_translation(tile.pos.as_vec3() + (Vec3::ONE - Vec3::Y) * 0.5)
//                             .with_scale(Vec3::ONE * 0.49 * if pat.is_flipped { -1.0 } else { 1.0 })
//                             .with_rotation(Quat::from_axis_angle(Vec3::Y, f32::to_radians(pat.rotation as f32 * 90.0))),
//                         ..default()
//                     },
//                     WfcTest,
//                 ));
//             }
//         }
//     });
// }