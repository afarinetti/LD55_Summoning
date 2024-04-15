use bevy::prelude::*;

// ----- Classes (they're really just callback functions that modify bundles / text styles, but it's useful to think of them as .css classes) -----
pub fn c_root(b: &mut NodeBundle) {
    b.style.width = Val::Percent(100.);
    b.style.height = Val::Percent(100.);
    b.style.align_items = AlignItems::Center;
    b.style.justify_content = JustifyContent::Center;
    b.style.flex_direction = FlexDirection::Column;
}

// pub fn c_half(b: &mut NodeBundle) {
//     let s = &mut b.style;
//     s.width = Val::Percent(50.);
//     s.height = Val::Percent(100.);
//     s.flex_direction = FlexDirection::Column;
//     s.justify_content = JustifyContent::Center;
//     s.align_items = AlignItems::Center;
//     s.padding = UiRect::all(Val::Px(10.));
// }

pub fn c_no_bg(_b: &mut NodeBundle) {
    // no background
}

// pub fn c_green(b: &mut NodeBundle) {
//     b.background_color = Color::rgb_u8(125, 212, 148).into();
// }
//
// pub fn c_blue(b: &mut NodeBundle) {
//     b.background_color = Color::rgb_u8(125, 164, 212).into();
// }

pub fn c_text(_a: &AssetServer, b: &mut TextBundle) {
    b.style.margin = UiRect::all(Val::Px(10.));
}

pub fn c_button(assets: &AssetServer, b: &mut ButtonBundle) {
    let s = &mut b.style;
    s.width = Val::Px(90.0);
    s.height = Val::Px(28.0);
    s.justify_content = JustifyContent::Center;
    s.align_items = AlignItems::Center;
    b.background_color = Color::rgb_u8(66, 135, 245).into();
    b.image = assets.load("button.png").into();
}

pub fn c_pixel_title(assets: &AssetServer, s: &mut TextStyle) {
    s.font = assets.load("fonts/prstartk.ttf");
    s.font_size = 24.0;
    s.color = Color::WHITE;
}

pub fn c_pixel_button(assets: &AssetServer, s: &mut TextStyle) {
    s.font = assets.load("fonts/prstartk.ttf");
    s.font_size = 14.0;
    s.color = Color::WHITE;
}
