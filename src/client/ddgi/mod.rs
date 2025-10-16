use bevy::prelude::*;
use bevy::render::{
    render_asset::{RenderAssetUsages},
    render_resource::*,
    renderer::{RenderDevice},
    RenderApp, Render,
};

use crate::prelude::*;
use crate::client::prelude::*;

const MAX_LIGHT_LEVEL: i32 = 15;

#[cfg(not(feature = "ddgi2"))]
mod ddgi1_plugin;

#[cfg(feature = "ddgi2")]
mod ddgi2_plugin;

#[cfg(not(feature = "ddgi2"))]
pub use ddgi1_plugin::{
    DDGI1Plugin as DDGIPlugin, DDGI1Uniforms as DDGIUniforms,
};

#[cfg(feature = "ddgi2")]
pub use ddgi2_plugin::{
    DDGI2Plugin as DDGIPlugin, DDGI2Uniforms as DDGIUniforms,
};