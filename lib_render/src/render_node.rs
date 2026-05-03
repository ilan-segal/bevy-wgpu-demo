use std::ops::Deref;

use bevy::ecs::query::QueryData;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_graph::{RenderGraphContext, ViewNode};
use bevy::render::render_resource::{
    IndexFormat, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp,
};
use bevy::render::renderer::RenderContext;
use bevy::render::view::ViewTarget;
use bevy::{prelude::*, render::renderer::RenderQueue};

use crate::pipeline::{
    GlobalsUniformBindGroup, GlobalsUniformBuffer, IndexBuffer, MainPassDepth, MyShadowMapPipeline,
    ShadowMapTextureBindGroup, ShadowPassDepth, ShadowPassGlobalsUniformBindGroup,
    ShadowPassGlobalsUniformBuffer,
};
use crate::texture::TextureBindGroup;
use crate::vertex::VertexBuffer;
use crate::{InstanceBuffer, InstanceBuffers};
use crate::{
    globals::{AmbientLight, CameraData, DirectionalLight, FogSettings, GlobalsData, StartupTime},
    pipeline::MyRenderPipeline,
};

#[derive(bevy::render::render_graph::RenderLabel, Hash, Clone, Debug, PartialEq, Eq)]
pub struct MyRenderNodeLabel;

#[derive(Default)]
pub struct MyRenderNode;

impl ViewNode for MyRenderNode {
    // choose the appropriate ViewQuery type for your node; `()` is a no-op
    type ViewQuery = ();

    fn update(&mut self, world: &mut World) {
        if !world.contains_resource::<MyRenderPipeline>() {
            return;
        }
        // Update globals buffer
        let CameraData {
            projection_matrix,
            position: camera_position,
        } = world.resource::<CameraData>();
        let StartupTime(startup_time) = world.resource::<StartupTime>();
        let elapsed_seconds = startup_time.elapsed().as_secs_f32();

        let mut globals = GlobalsData::default();
        globals.elapsed_seconds = elapsed_seconds;
        globals.projection_matrix = projection_matrix.to_cols_array_2d();
        globals.camera_position = camera_position.to_array();
        if let Some(AmbientLight(colour)) = world.get_resource::<AmbientLight>() {
            globals.ambient_light = colour.to_srgba().to_f32_array_no_alpha();
        }
        if let Some(directional_light) = world.get_resource::<DirectionalLight>() {
            globals.directional_light = directional_light.color.to_srgba().to_f32_array_no_alpha();
            globals.directional_light_direction = directional_light.direction.to_array();
            const SHADOW_SIZE: f32 = 128.0;
            const NEGATIVE_Z: Mat4 = Mat4::from_cols_array_2d(&[
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., -1., 0.],
                [0., 0., 1., 1.],
            ]);
            let shadow_projection = NEGATIVE_Z
                * Mat4::orthographic_rh(
                    -SHADOW_SIZE,
                    SHADOW_SIZE,
                    -SHADOW_SIZE,
                    SHADOW_SIZE,
                    -SHADOW_SIZE * 2.,
                    SHADOW_SIZE * 2.,
                )
                * Transform::from_translation(globals.camera_position.into())
                    .looking_to(directional_light.direction, Vec3::Y)
                    .compute_matrix()
                    .inverse();
            globals.shadow_map_projection = shadow_projection.to_cols_array_2d();
        }
        if let Some(fog_settings) = world.get_resource::<FogSettings>() {
            globals.fog_color = fog_settings.color.to_linear().to_f32_array_no_alpha();
            globals.fog_b = fog_settings.b;
        }

        let render_queue = world.resource::<RenderQueue>();
        let buffer = world.resource::<GlobalsUniformBuffer>();
        render_queue.write_buffer(&buffer.buffer, 0, bytemuck::bytes_of(&globals));

        let mut shadow_pass_globals = globals.clone();
        shadow_pass_globals.projection_matrix = globals.shadow_map_projection;
        let shadow_pass_buffer = world.resource::<ShadowPassGlobalsUniformBuffer>();
        render_queue.write_buffer(
            &shadow_pass_buffer.buffer,
            0,
            bytemuck::bytes_of(&shadow_pass_globals),
        );
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext<'_>,
        render_context: &mut RenderContext<'_>,
        _view_query: <Self::ViewQuery as QueryData>::Item<'_>,
        world: &'_ World,
    ) -> std::result::Result<(), bevy::render::render_graph::NodeRunError> {
        if !world.contains_resource::<MyRenderPipeline>() {
            return Ok(());
        }
        let shadow_pipeline = world.resource::<MyShadowMapPipeline>();
        let shadow_depth = world.resource::<ShadowPassDepth>();
        let main_pipeline = world.resource::<MyRenderPipeline>();
        let VertexBuffer { vertex_buffer, .. } = world.resource::<VertexBuffer>();
        let IndexBuffer {
            buffer: index_buffer,
            num_indices,
        } = world.resource::<IndexBuffer>();
        let depth = world.resource::<MainPassDepth>();

        let Some(mut query) =
            world.try_query_filtered::<(&ViewTarget, &ExtractedCamera), With<Camera>>()
        else {
            panic!("Failed query for view target and extracted camera");
        };

        let GlobalsUniformBindGroup {
            bind_group: globals_uniform_bind_group,
        } = world.resource::<GlobalsUniformBindGroup>();
        let ShadowPassGlobalsUniformBindGroup {
            bind_group: shadow_pass_globals_uniform_bind_group,
        } = world.resource::<ShadowPassGlobalsUniformBindGroup>();
        let TextureBindGroup {
            bind_group: texture_bind_group,
            ..
        } = world.resource::<TextureBindGroup>();
        let ShadowMapTextureBindGroup {
            bind_group: shadow_map_bind_group,
            ..
        } = world.resource::<ShadowMapTextureBindGroup>();

        for (view_target, _cam) in query.iter(&world) {
            let shadow_pass_desc = RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &shadow_depth.0.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            {
                let mut shadow_pass = render_context
                    .command_encoder()
                    .begin_render_pass(&shadow_pass_desc);
                shadow_pass.set_pipeline(&shadow_pipeline.pipeline);
                shadow_pass.set_bind_group(0, shadow_pass_globals_uniform_bind_group, &[]);
                shadow_pass.set_index_buffer(*index_buffer.slice(..).deref(), IndexFormat::Uint16);
                shadow_pass.set_vertex_buffer(0, *vertex_buffer.slice(..).deref());

                for (
                    pos,
                    InstanceBuffer {
                        buffer: instance_buffer,
                        num_instances,
                    },
                ) in world
                    .resource::<InstanceBuffers>()
                    .chunk_pos_to_buffer
                    .iter()
                {
                    if num_instances == &0 {
                        continue;
                    }
                    let chunk_pos_array = pos.to_array();
                    shadow_pass.set_push_constants(
                        bevy::render::render_resource::ShaderStages::VERTEX,
                        0, // offset
                        bytemuck::cast_slice(&[chunk_pos_array]),
                    );
                    shadow_pass.set_vertex_buffer(1, *instance_buffer.slice(..).deref());
                    shadow_pass.draw_indexed(0..*num_indices, 0, 0..*num_instances);
                }
            }

            let view = view_target.main_texture_view();
            let color_attachment = RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(crate::SKY_COLOR.to_linear().into()),
                    store: StoreOp::Store,
                },
            };

            let desc = RenderPassDescriptor {
                label: Some("triangle_pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.0.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            {
                let mut pass = render_context.command_encoder().begin_render_pass(&desc);
                pass.set_pipeline(&main_pipeline.pipeline);
                pass.set_bind_group(0, globals_uniform_bind_group, &[]);
                pass.set_bind_group(1, texture_bind_group, &[]);
                pass.set_bind_group(2, shadow_map_bind_group, &[]);
                pass.set_index_buffer(*index_buffer.slice(..).deref(), IndexFormat::Uint16);
                pass.set_vertex_buffer(0, *vertex_buffer.slice(..).deref());

                for (
                    pos,
                    InstanceBuffer {
                        buffer: instance_buffer,
                        num_instances,
                    },
                ) in world
                    .resource::<InstanceBuffers>()
                    .chunk_pos_to_buffer
                    .iter()
                {
                    if num_instances == &0 {
                        continue;
                    }
                    let chunk_pos_array = pos.to_array();
                    pass.set_push_constants(
                        bevy::render::render_resource::ShaderStages::VERTEX,
                        0, // offset
                        bytemuck::cast_slice(&[chunk_pos_array]),
                    );
                    pass.set_vertex_buffer(1, *instance_buffer.slice(..).deref());
                    pass.draw_indexed(0..*num_indices, 0, 0..*num_instances);
                }
            }
        }

        Ok(())
    }
}
