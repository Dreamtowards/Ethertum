

use bevy_egui::egui::Painter;

use crate::{item::{Inventory, ItemStack}, ui::prelude::*};

pub fn ui_holding_item() -> &'static mut ItemStack {
    static mut CURR_HOLD: ItemStack = ItemStack { count: 0, item_id: 0 };
    unsafe { &mut CURR_HOLD }
}

pub fn draw_ui_holding_item(mut ctx: EguiContexts,) {
    let hold = ui_holding_item();

    if !hold.is_empty() {
        let curpos = ctx.ctx_mut().pointer_latest_pos().unwrap();
        let size = vec2(50., 50.);

        draw_item(&hold, Rect::from_min_size(curpos-size/2., size), &ctx.ctx_mut().debug_painter());
    }
}

pub fn draw_item(slot: &ItemStack, rect: Rect, painter: &Painter) {
    let reg = unsafe { &*crate::item::_ITEMS_REG };
    let num_all_items = reg.reg.len();

    // Item Texture
    let uv_siz = 1. / num_all_items as f32;
    let uv_x = uv_siz * (slot.item_id - 1) as f32;
    painter.image(reg.atlas_egui, rect.shrink(3.), 
        Rect::from_min_size(pos2(uv_x, 0.), vec2(uv_siz, 1.)), Color32::WHITE);
    // Item Count
    painter.text(rect.max - vec2(4., 2.), Align2::RIGHT_BOTTOM, 
        slot.count.to_string(), egui::FontId::proportional(12.), Color32::from_gray(190));
}

pub fn ui_item_stack(ui: &mut egui::Ui, slot: &mut ItemStack) {
    let reg = unsafe { &*crate::item::_ITEMS_REG };
    let num_all_items = reg.reg.len();

    let slot_btn = egui::Button::new("").fill(Color32::from_black_alpha(100));
    // if cli.hotbar_index == i {
    //     slot = slot.stroke(Stroke::new(3., Color32::WHITE));
    // }
    
    let slot_btn_size = 50.;
    let mut resp = ui.add_sized([slot_btn_size, slot_btn_size], slot_btn);
    
    if !slot.is_empty() {
        // Tooltip
        resp = resp.on_hover_ui(|ui| {
            if let Some(name) = reg.reg.at((slot.item_id - 1) as u16) {

                ui.label(name);
                ui.small(format!("{} [{}/{}] x{}", name, slot.item_id, num_all_items, slot.count));
            }
        });

        draw_item(slot, resp.rect, ui.painter())
    }

    let hold = ui_holding_item();
        
    if resp.clicked() {
        ItemStack::swap(hold, slot);
    } else if resp.secondary_clicked() {

        crate::util::as_mut(slot).count += 1;
        crate::util::as_mut(slot).item_id += 1;
    }

}

pub fn ui_inventory(ui: &mut egui::Ui, inv: &mut Inventory) -> InnerResponse<()> {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
        ui.style_mut().spacing.item_spacing = vec2(4., 4.);

        for item in inv.items.iter_mut() {
            ui_item_stack(ui, item);
        }
    })
}
