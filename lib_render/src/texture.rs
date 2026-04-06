use std::marker::PhantomData;

use bevy::{
    prelude::*,
    render::render_resource::{
        AddressMode, BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType,
        FilterMode, SamplerBindingType, ShaderStages, TextureSampleType, TextureUsages,
        TextureViewDimension,
    },
};
use strum::IntoEnumIterator;

pub trait TextureIndex {
    fn get_name(&self) -> &'static str;
}

pub(crate) struct TexturePlugin<TerrainType> {
    _phantom: PhantomData<TerrainType>,
}

impl<T> TexturePlugin<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<TerrainType: 'static + IntoEnumIterator + TextureIndex + Send + Sync> Plugin
    for TexturePlugin<TerrainType>
{
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_terrain_colors::<TerrainType>)
            .sub_app_mut(bevy::render::RenderApp)
            .add_systems(
                ExtractSchedule,
                prepare_texture_bind_group::<TerrainType>
                    .run_if(not(resource_exists::<TextureBindGroup>)),
            );
    }
}

#[derive(Resource)]
struct TerrainColorTextureHandles {
    handles: Vec<Handle<Image>>,
}

#[derive(Resource)]
pub struct TerrainColorTextureIndices {
    indices_by_name: std::collections::HashMap<&'static str, usize>,
}

impl TerrainColorTextureIndices {
    pub fn get_index<T: TextureIndex>(&self, terrain_type: &T) -> Option<&usize> {
        let name = terrain_type.get_name();
        self.indices_by_name.get(name)
    }
}

fn load_terrain_colors<TerrainType: 'static + IntoEnumIterator + TextureIndex + Send + Sync>(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let handles = TerrainType::iter()
        .map(|t| t.get_name())
        .map(|name| asset_server.load(format!("{name}.png")))
        .collect();
    commands.insert_resource(TerrainColorTextureHandles { handles });
    let indices_by_name = TerrainType::iter()
        .enumerate()
        .map(|(i, ty)| (ty.get_name(), i))
        .collect();
    commands.insert_resource(TerrainColorTextureIndices { indices_by_name });
}

#[derive(Resource)]
pub(crate) struct TextureBindGroup {
    pub bind_group: bevy::render::render_resource::BindGroup,
    pub layout: bevy::render::render_resource::BindGroupLayout,
}

fn prepare_texture_bind_group<TerrainType: Send + Sync + TextureIndex>(
    mut commands: Commands,
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    texture_handles: bevy::render::Extract<Res<TerrainColorTextureHandles>>,
    render_device: Res<bevy::render::renderer::RenderDevice>,
    render_queue: Res<bevy::render::renderer::RenderQueue>,
    image_assets: bevy::render::Extract<Res<Assets<Image>>>,
) {
    let image_layers = texture_handles
        .handles
        .iter()
        .flat_map(|handle| gpu_images.get(handle))
        .collect::<Vec<_>>();
    if image_layers.len() != texture_handles.handles.len() {
        return;
    }
    info!("Loaded GPU images. Creating texture array.");

    let layer_count = image_layers.len() as u32;
    let extent = bevy::render::render_resource::Extent3d {
        depth_or_array_layers: layer_count,
        ..image_layers[0].size
    };
    let array_texture =
        render_device.create_texture(&bevy::render::render_resource::TextureDescriptor {
            label: Some("terrain_color_texture_array"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: image_layers[0].texture_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

    for (i, img) in image_layers.iter().enumerate() {
        let data = image_assets
            .get(texture_handles.handles[i].id())
            .cloned()
            .expect("Texture should exist in CPU land")
            .data;
        let data = data.unwrap().clone();
        let data = data.as_slice();
        render_queue.write_texture(
            bevy::render::render_resource::TexelCopyTextureInfo {
                texture: &array_texture,
                mip_level: 0,
                origin: bevy::render::render_resource::Origin3d {
                    x: 0,
                    y: 0,
                    z: i as _,
                },
                aspect: bevy::render::render_resource::TextureAspect::All,
            },
            data,
            bevy::render::render_resource::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(img.size.width * 4),
                rows_per_image: None,
            },
            bevy::render::render_resource::Extent3d {
                depth_or_array_layers: 1,
                ..img.size
            },
        );
    }

    let layout = render_device.create_bind_group_layout(
        Some("my texture bind group layout"),
        &[
            // Texture binding
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    );
    let nearest_sampler =
        render_device.create_sampler(&bevy::render::render_resource::SamplerDescriptor {
            label: Some("nearest_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        });

    // Create view, sampler, and bind group
    let texture_view =
        array_texture.create_view(&bevy::render::render_resource::TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

    let bind_group = render_device.create_bind_group(
        Some("My texture bind group"),
        &layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&nearest_sampler),
            },
        ],
    );

    commands.insert_resource(TextureBindGroup { bind_group, layout });
}
