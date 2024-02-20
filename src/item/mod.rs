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

        app.add_systems(PostStartup, bake_items);
    }
}


#[derive(Resource, Default)]
struct Items {
    pub registry: Registry,

    // pub apple: RegId,
    // pub pickaxe: RegId,
    // pub stick: RegId,
    // pub shear: RegId,

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

fn bake_items(
    mut items: ResMut<Items>,
) {
    items.registry.build_num_id();
    info!("Registered {} items: {:?}", items.registry.len(), items.registry.vec);


    // build atlas

    
}