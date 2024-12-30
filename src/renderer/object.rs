use super::{
    context::Context,
    hypershape::{Hypershape, HypershapeDescriptor},
    material::MaterialStore,
    model::{load_model, Mesh, Model},
    pipeline::PipelineStore,
    shader::ShaderType,
    texture::TextureStore,
};

use cgmath::{Deg, Matrix4, Quaternion, Rotation3, SquareMatrix, Vector3};
use wgpu::util::DeviceExt;

use std::path::PathBuf;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    matrix: [[f32; 4]; 4],
    inverse_matrix: [[f32; 4]; 4],
}

impl TransformUniform {
    pub fn new() -> Self {
        Self {
            matrix: Matrix4::identity().into(),
            inverse_matrix: Matrix4::identity().into(),
        }
    }

    pub fn calculate(
        &mut self,
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        scale: Vector3<f32>,
    ) {
        let matrix = Matrix4::from_translation(position)
            * Matrix4::from(rotation)
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
        self.matrix = matrix.into();
        self.inverse_matrix = matrix.invert().unwrap().into();
    }
}

pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
    pub uniform: TransformUniform,
}

#[allow(dead_code)]
impl Transform {
    pub fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            uniform: TransformUniform::new(),
        }
    }

    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            uniform: TransformUniform::new(),
        }
    }

    pub fn update_uniform(&mut self) {
        self.uniform
            .calculate(self.position, self.rotation, self.scale);
    }
}

pub enum ObjectData {
    Model(Model),
    Hyper(Hypershape),
}

#[allow(dead_code)]
pub struct Object {
    data: ObjectData,
    pub transform: Transform,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Object {
    pub fn new(
        data: ObjectData,
        transform: Transform,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Uniform Buffer"),
            contents: bytemuck::cast_slice(&[transform.uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("object_bind_group"),
        });

        Self {
            data,
            transform,
            uniform_buffer,
            bind_group,
        }
    }

    pub async fn from_model_path(
        model_path: &PathBuf,
        context: &Context<'_>,
        material_store: &mut MaterialStore,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let model = load_model(model_path, &context.device, material_store)
            .await
            .unwrap();

        Self::new(
            ObjectData::Model(model),
            Transform::default(),
            &context.device,
            &bind_group_layout,
        )
    }

    pub fn from_hyper(
        descriptor: &HypershapeDescriptor,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let hypershape = Hypershape::new(*descriptor, device, bind_group_layout);
        Self::new(
            ObjectData::Hyper(hypershape),
            Transform::default(),
            device,
            bind_group_layout,
        )
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.transform.update_uniform();
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.transform.uniform]),
        );
    }
}

fn deduce_pipeline<'a>(
    material_id: u32,
    material_store: &'a MaterialStore,
    pipeline_store: &'a PipelineStore,
) -> &'a wgpu::RenderPipeline {
    let material = material_store.get_material(material_id);
    match material.shader_type {
        ShaderType::Basic => &pipeline_store.basic,
        ShaderType::Grid => &pipeline_store.grid,
        ShaderType::Hyper => &pipeline_store.hyper,
    }
}

pub trait DrawObject<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        diffuse_bind_group: &'a wgpu::BindGroup,
        translation_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_hyper(
        &mut self,
        camera_bind_group: &'a wgpu::BindGroup,
        translation_bind_group: &'a wgpu::BindGroup,
        hypershape_bind_group: &'a wgpu::BindGroup,
        slice_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_object(
        &mut self,
        object: &'a Object,
        camera_bind_group: &'a wgpu::BindGroup,
        material_store: &'a MaterialStore,
        texture_store: &'a TextureStore,
        pipeline_store: &'a PipelineStore,
        slice_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawObject<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        diffuse_bind_group: &'b wgpu::BindGroup,
        translation_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &camera_bind_group, &[]);
        self.set_bind_group(1, &diffuse_bind_group, &[]);
        self.set_bind_group(2, &translation_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, 0..1);
    }

    fn draw_hyper(
        &mut self,
        camera_bind_group: &'b wgpu::BindGroup,
        translation_bind_group: &'b wgpu::BindGroup,
        hypershape_bind_group: &'b wgpu::BindGroup,
        slice_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_bind_group(0, &camera_bind_group, &[]);
        self.set_bind_group(1, &translation_bind_group, &[]);
        self.set_bind_group(2, &hypershape_bind_group, &[]);
        self.set_bind_group(3, &slice_bind_group, &[]);
        self.draw(0..3, 0..1);
    }

    fn draw_object(
        &mut self,
        object: &'a Object,
        camera_bind_group: &'a wgpu::BindGroup,
        material_store: &'a MaterialStore,
        texture_store: &'a TextureStore,
        pipeline_store: &'a PipelineStore,
        slice_bind_group: &'a wgpu::BindGroup,
    ) {
        if let ObjectData::Model(model) = &object.data {
            for data in &model.data {
                let mesh = &data.mesh;
                let material = material_store.get_material(data.material_id);
                let diffuse = texture_store.get_texture(material.diffuse_texture_id);
                self.set_pipeline(deduce_pipeline(
                    data.material_id,
                    material_store,
                    pipeline_store,
                ));
                self.draw_mesh(
                    mesh,
                    camera_bind_group,
                    diffuse.bind_group.as_ref().unwrap(),
                    &object.bind_group,
                );
            }
        } else if let ObjectData::Hyper(hyper) = &object.data {
            self.set_pipeline(&pipeline_store.hyper);
            self.draw_hyper(
                camera_bind_group,
                &object.bind_group,
                &hyper.bind_group,
                slice_bind_group,
            );
        }
    }
}


#[allow(dead_code)]
pub struct ObjectManager {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub hyper_bind_group_layout: wgpu::BindGroupLayout,
    actors: Vec<Object>,            // User-defined objects
    hyper_actors: Vec<Object>,      // User-defined hyper-dimentional objects
    immutable_objects: [Object; 1], // Static objects
}

#[allow(dead_code)]
impl<'a> ObjectManager {
    pub async fn new(context: &'a Context<'a>, material_store: &'a mut MaterialStore) -> Self {
        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                });

        let hyper_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                });

        // ToDo Delegate creation of immutable objects to a separate function
        let mut grid = Object::from_model_path(
            &PathBuf::from("models/plane.obj"),
            context,
            material_store,
            &bind_group_layout,
        )
        .await;
        grid.transform.scale = Vector3::new(100.0, 100.0, 100.0);
        if let ObjectData::Model(model) = &mut grid.data {
            for data in &mut model.data {
                data.material_id = 1;
            }
        }

        Self {
            bind_group_layout,
            hyper_bind_group_layout,
            actors: Vec::new(),
            hyper_actors: Vec::new(),
            immutable_objects: [grid],
        }
    }

    pub fn add_actor(&mut self, object: Object) {
        self.actors.push(object);
    }

    pub fn remove_actor(&mut self, index: usize) {
        self.actors.remove(index);
    }

    pub async fn create_actor(
        &mut self,
        model_path: &PathBuf,
        context: &Context<'_>,
        material_store: &mut MaterialStore,
    ) -> &mut Object {
        let actor =
            Object::from_model_path(model_path, context, material_store, &self.bind_group_layout)
                .await;
        self.add_actor(actor);
        self.actors.last_mut().unwrap()
    }

    pub fn create_hyper_actor(
        &mut self,
        descriptor: &HypershapeDescriptor,
        context: &Context<'_>,
    ) -> &mut Object {
        let actor = Object::from_hyper(descriptor, &context.device, &self.hyper_bind_group_layout);
        self.hyper_actors.push(actor);
        self.hyper_actors.last_mut().unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.actors.iter().chain(
            self.hyper_actors
                .iter()
                .chain(self.immutable_objects.iter()),
        )
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Object> {
        self.actors.iter_mut().chain(
            self.hyper_actors
                .iter_mut()
                .chain(self.immutable_objects.iter_mut()),
        )
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        for object in self.iter_mut() {
            object.update(queue);
        }
    }
}
