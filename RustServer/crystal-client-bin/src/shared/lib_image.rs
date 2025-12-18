use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crystal_lib::LibFile;

pub(crate) fn lib_to_image_handle(
    lib: &LibFile,
    index: usize,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, crystal_lib::LibError> {
    let img = lib.get_image(index)?;
    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: img.width as u32,
        height: img.height as u32,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();
    Ok(images.add(bevy_img))
}

pub(crate) fn lib_to_sprite(
    lib: &LibFile,
    index: usize,
    images: &mut Assets<Image>,
) -> Result<(Handle<Image>, u32, u32), crystal_lib::LibError> {
    let img = lib.get_image(index)?;
    let w = img.width as u32;
    let h = img.height as u32;
    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();
    Ok((images.add(bevy_img), w, h))
}

pub(crate) fn lib_to_sprite_with_offset(
    lib: &LibFile,
    index: usize,
    images: &mut Assets<Image>,
) -> Result<(Handle<Image>, u32, u32, i16, i16), crystal_lib::LibError> {
    let img = lib.get_image(index)?;
    let w = img.width as u32;
    let h = img.height as u32;
    let x = img.x;
    let y = img.y;
    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();
    Ok((images.add(bevy_img), w, h, x, y))
}
