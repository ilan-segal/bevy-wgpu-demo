#[derive(bevy::render::render_graph::RenderLabel, Hash, Clone, Debug, PartialEq, Eq)]
pub struct MyRenderNodeLabel;

#[derive(Default)]
pub struct MyRenderNode;

impl bevy::render::render_graph::ViewNode for MyRenderNode {
    type ViewQuery = ();

    fn update(&mut self, _world: &mut bevy::ecs::world::World) {
        todo!()
    }

    fn run<'w>(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        view_query: bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w bevy::ecs::world::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        todo!()
    }
}
