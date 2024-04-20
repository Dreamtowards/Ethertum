use bevy::{
    math::{ivec3, vec3},
    prelude::*,
};
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
            sockets: sockets,
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
