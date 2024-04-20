use crate::util::registry::{RegId, Registry};

// pub struct Item {
//     // tab_category
//     max_stacksize: u32,

//     max_damage: u32,
//     // name
// }

pub struct ItemStack {
    pub count: u8,
    pub item_id: u8,
    // pub durability
}
impl ItemStack {
    pub fn new(count: u8, item: u8) -> Self {
        Self {
            count,
            item_id: item,
        }
    }
}

#[derive(Default)]
pub struct Inventory {
    pub items: Vec<ItemStack>,
}

impl Inventory {
    pub fn new(size: usize) -> Self {
        Self {
            items: Vec::with_capacity(size),
        }
    }
}

use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Items::default());
        // app.insert_resource(Registry::default());

        app.add_systems(Startup, setup_items);

        // app.add_systems(PostStartup, bake_items);
    }
}

#[derive(Resource, Default)]
pub struct Items {
    pub reg: Registry,
    pub atlas: Handle<Image>,

    pub apple: RegId,

    pub coal: RegId,
    pub stick: RegId,

    pub frame: RegId,
    pub lantern: RegId,

    pub pickaxe: RegId,
    pub shears: RegId,
    pub grapple: RegId,
    pub iron_ingot: RegId,
}

pub static mut _UI_ITEMS_ATLAS: bevy_egui::egui::TextureId = bevy_egui::egui::TextureId::Managed(0);
pub static mut _NUM_ITEMS: usize = 8;

fn setup_items(
    mut items: ResMut<Items>, 
    // mut reg: ResMut<Registry>, 
    asset_server: Res<AssetServer>,
    mut egui_ctx: bevy_egui::EguiContexts,
) {
    let reg = crate::util::as_mut(&items.reg);
    let items = crate::util::as_mut(&*items);
    // Food
    items.apple = reg.insert("apple");
     reg.insert("avocado");  // tmp

    // Material
    items.coal = reg.insert("coal");
    items.stick = reg.insert("stick");

    // Object
    items.frame = reg.insert("frame");
    items.lantern = reg.insert("lantern");
    // torch

    // Tool
    items.pickaxe = reg.insert("pickaxe");
    // shovel
    items.shears = reg.insert("shears");
    items.grapple = reg.insert("grapple");
    items.iron_ingot = reg.insert("iron_ingot");

    // below are temporary. Build should defer to PostStartup stage.:

    // Build NumId Table
    reg.build_num_id();
    info!("Registered {} items: {:?}", reg.len(), reg.vec);

    items.atlas = asset_server.load("baked/items.png");

    unsafe {
        _UI_ITEMS_ATLAS = egui_ctx.add_image(items.atlas.clone());
        _NUM_ITEMS =  reg.len();
    }
}

// use image::{self, GenericImageView, RgbaImage};

// fn bake_items(
//     mut items: ResMut<Items>,
//     asset_server: Res<AssetServer>,
// ) -> anyhow::Result<()> {

// // Generate Items Atlas Image
// let cache_file = std::env::current_dir()?.join("baked/items.png");
// let resolution = 64;

// if let Err(_) = std::fs::metadata(&cache_file) {
//     info!("Items Atlas Image cache not found, Generating...");

//     let n = items.registry.len() as u32;

//     let mut atlas = RgbaImage::new(n * resolution, resolution);

//     for (idx, str_id) in items.registry.vec.iter().enumerate() {
//         let idx = idx as u32;

//         let imgloc = if false {
//             // todo: ASSET_ROOT_PATH
//             format!("assets/textures/{str_id}/view.png")
//         } else {
//             format!("assets/items/{str_id}/view.png")
//         };

//         let img = image::open(imgloc)?;
//         let img = img.resize_exact(resolution, resolution, image::imageops::FilterType::Triangle);

//         // copy to
//         for y in 0..resolution {
//             for x in 0..resolution {
//                 atlas.put_pixel(idx*resolution + x, y, img.get_pixel(x, y));
//             }
//         }
//     }

//     std::fs::create_dir_all(&cache_file.parent().ok_or(crate::err_opt_is_none!())?)?;
//     atlas.save(&cache_file)?;
// }

// items.atlas = asset_server.load(cache_file);
// Ok(())
// }

// fn gen_items_atlas_image(cache_file: &str, resolution: u32) {

// }
