use crate::util::registry::{RegId, Registry};

struct Item {

    // tab_category

    max_stacksize: u32,

    max_damage: u32,

    // name
}



struct ItemStack {
    pub count: u8,
    pub item_id: u8,

    // pub durability
}

struct Inventory {
    pub items: Vec<ItemStack>,

}


use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {

        app.insert_resource(Items::default());
        
        app.add_systems(Startup, setup_items);

        app.add_systems(PostStartup, bake_items.pipe(error_handler));
    }
}


#[derive(Resource, Default)]
struct Items {
    pub registry: Registry,
    pub atlas: Handle<Image>,

    // pub apple: RegId,
    // pub pickaxe: RegId,
    // pub stick: RegId,
    // pub shear: RegId,

}

fn error_handler(In(result): In<anyhow::Result<()>>) {
    let hm = crate::hashmap![
        "foo" => 100,
        "bar" => 200,
    ];

    if let Err(err) = result {
        panic!("{}", err)
    }
}

fn setup_items(
    mut items: ResMut<Items>,
) {
    let reg = &mut items.registry;

    // Food
    reg.insert("apple");
    reg.insert("avocado");

    // Material
    reg.insert("coal");
    reg.insert("stick");

    // Object
    reg.insert("frame");
    reg.insert("lantern");
    // torch

    // Tool
    reg.insert("pickaxe");
    // shovel
    reg.insert("shears");
    reg.insert("grapple");
    reg.insert("iron_ingot");


}


use image::{self, GenericImageView, RgbaImage};

fn bake_items(
    mut items: ResMut<Items>,
    asset_server: Res<AssetServer>,
) -> anyhow::Result<()> {
    // Build NumId Table
    items.registry.build_num_id();
    info!("Registered {} items: {:?}", items.registry.len(), items.registry.vec);

    // Generate Items Atlas Image
    let cache_file = std::env::current_dir()?.join("cache/items.png");
    let resolution = 64;

    if let Err(_) = std::fs::metadata(&cache_file) {
        info!("Items Atlas Image cache not found, Generating...");

        let n = items.registry.len() as u32;

        let mut atlas = RgbaImage::new(n * resolution, resolution);
    
        for (idx, str_id) in items.registry.vec.iter().enumerate() {
            let idx = idx as u32;
            
            let imgloc = if false { 
                // todo: ASSET_ROOT_PATH
                format!("assets/textures/{str_id}/view.png")
            } else {
                format!("assets/items/{str_id}/view.png")
            };
    
            let img = image::open(imgloc)?;
            let img = img.resize_exact(resolution, resolution, image::imageops::FilterType::Triangle);
    
            // copy to
            for y in 0..resolution {
                for x in 0..resolution {
                    atlas.put_pixel(idx*resolution + x, y, img.get_pixel(x, y));
                }
            }
        }
    
        std::fs::create_dir_all(&cache_file.parent().ok_or(crate::err_opt_is_none!())?)?;
        atlas.save(&cache_file)?;
    }

    items.atlas = asset_server.load(cache_file);
    Ok(())
}


fn gen_items_atlas_image(cache_file: &str, resolution: u32) {

}