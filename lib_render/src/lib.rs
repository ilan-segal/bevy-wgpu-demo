use bevy::{prelude::*, render::render_graph::RenderGraphApp};

use crate::render_node::{MyRenderNode, MyRenderNodeLabel};

mod camera;
mod render_node;
mod texture;

pub struct MyRenderPlugin;

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            // .add_observer(emit_chunk_despawn_event)
            // .add_event::<ChunkDespawn>()
            .sub_app_mut(bevy::render::RenderApp);
        // .init_resource::<StartupTime>()
        // .init_resource::<CameraData>()
        // .init_resource::<PipelineIsNotInitialized>()
        // .init_resource::<InstanceBuffers>()
        // .add_systems(
        //     ExtractSchedule,
        // (
        // (
        //     prepare_texture_bind_group,
        //     init_vertex_buffer,
        //     init_pipeline,
        // )
        //     .chain()
        //     .run_if(resource_exists::<PipelineIsNotInitialized>),
        // update_camera_data,
        // (remove_buffer_for_despawned_chunk, update_instance_buffer).chain(),
        // resize_depth_texture,
        // extract_resource_to_render_world::<AmbientLight>,
        // extract_resource_to_render_world::<DirectionalLight>,
        // extract_resource_to_render_world::<FogSettings>,
        // ),
        // );

        // add our node (use ViewNodeRunner to run a ViewNode) to the Core2d graph,
        // and insert an ordering edge so our node runs before the UI subgraph.
        render_app
            .add_render_graph_node::<bevy::render::render_graph::ViewNodeRunner<MyRenderNode>>(bevy::core_pipeline::core_3d::graph::Core3d, MyRenderNodeLabel)
            // (Node2d::EndMainPassPostProcessing, MyCustomPassLabel, SubGraphUi)
            // means: EndMainPassPostProcessing -> MyCustomPass -> SubGraphUi
            .add_render_graph_edges(
                bevy::core_pipeline::core_3d::graph::Core3d,
                (
                    bevy::core_pipeline::core_3d::graph::Node3d::EndMainPassPostProcessing,
                    MyRenderNodeLabel,
                    bevy::ui::graph::NodeUi::UiPass,
                ),
            );
    }
}
