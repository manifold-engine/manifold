use wgpu::util::DeviceExt;

use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Zeroable)]
#[allow(dead_code)]
pub enum HypershapeType {
    Sphere = 0,
    Box = 1,
    Torus = 2,
}

impl Default for HypershapeType {
    fn default() -> Self {
        HypershapeType::Sphere
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HypershapeDescriptor {
    pub shape_type: u32,
    pub radius: f32,
    pub size: f32,
}

#[allow(dead_code)]
pub struct Hypershape {
    pub descriptor: HypershapeDescriptor, // uniform
    uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl Hypershape {
    pub fn new(
        descriptor: HypershapeDescriptor,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Uniform Buffer"),
            contents: bytemuck::cast_slice(&[descriptor]),
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
            descriptor,
            uniform_buffer,
            bind_group,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WSliceUniform {
    pub pos: f32,
}

pub struct HyperManager {
    pub slice_bind_group_layout: wgpu::BindGroupLayout,
    pub slice_bind_group: wgpu::BindGroup,
    pub slice_buffer: wgpu::Buffer,
    pub slice_uniform: WSliceUniform,
    pub movement_direction: f32,
}

impl HyperManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let slice_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("bind_group_layout"),
            });

        let slice_uniform = WSliceUniform { pos: 0.0 };

        let slice_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slice Buffer"),
            contents: bytemuck::cast_slice(&[slice_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let slice_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &slice_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: slice_buffer.as_entire_binding(),
            }],
            label: Some("slice_bind_group"),
        });

        Self {
            slice_bind_group_layout,
            slice_bind_group,
            slice_buffer,
            slice_uniform,
            movement_direction: 0.0,
        }
    }

    pub fn handle_slice_movement(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            self.movement_direction = 0.0;
            return;
        }
        match event.physical_key {
            PhysicalKey::Code(KeyCode::KeyK) => {
                self.movement_direction = 1.0;
            }
            PhysicalKey::Code(KeyCode::KeyJ) => {
                self.movement_direction = -1.0;
            }
            _ => {}
        };
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.slice_uniform.pos += self.movement_direction * 0.01;
        queue.write_buffer(
            &self.slice_buffer,
            0,
            bytemuck::cast_slice(&[self.slice_uniform]),
        );
    }
}
