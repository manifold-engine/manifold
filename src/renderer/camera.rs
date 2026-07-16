use cgmath::{
    perspective, Deg, InnerSpace, Matrix4, Point3, Quaternion, Rad, Rotation3, SquareMatrix,
    Vector3,
};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    pub eye: CameraEye,
    pub uniform: CameraUniform,
    pub controller: CameraController,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub buffer: Buffer,
}

impl Camera {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let eye = CameraEye {
            position: Point3::new(6.0, 3.0, 6.0),
            orientation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(45.0))
                * Quaternion::from_axis_angle(Vector3::unit_x(), Deg(-15.0)),
            up: Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fov: 45.0,
            near: 0.001,
            far: 100.0,
        };

        let controller = CameraController::new();

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&eye);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            eye: eye,
            uniform: uniform,
            controller: controller,
            bind_group_layout: bind_group_layout,
            bind_group: bind_group,
            buffer: buffer,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.controller.update_eye(&mut self.eye, 0.016);
        self.uniform.update_view_proj(&self.eye);

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}

pub struct CameraEye {
    pub position: Point3<f32>,
    pub orientation: Quaternion<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraEye {
    fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let forward = self.orientation * -Vector3::unit_z();
        let view = Matrix4::look_at_rh(self.position, self.position + forward, self.up);
        let proj = perspective(Deg(self.fov), self.aspect, self.near, self.far);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use SquareMatrix;
        Self {
            view_proj: Matrix4::identity().into(),
            inv_view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &CameraEye) {
        let proj = camera.build_view_projection_matrix();
        self.view_proj = proj.into();
        self.inv_view_proj = proj.invert().unwrap().into();
    }
}

pub struct CameraController {
    speed: f32,
    look_around: bool,
    direction_inputs: [bool; 4],
    mouse_delta: (f64, f64),
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            speed: 1.0,
            look_around: false,
            direction_inputs: [false; 4],
            mouse_delta: (0.0, 0.0),
        }
    }

    pub fn process_input_events(&mut self, event: &KeyEvent) -> bool {
        let is_pressed = event.state == ElementState::Pressed;
        match event.physical_key {
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                self.look_around = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyW) => {
                self.direction_inputs[0] = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) => {
                self.direction_inputs[1] = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyA) => {
                self.direction_inputs[2] = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) => {
                self.direction_inputs[3] = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse_delta(&mut self, delta: (f64, f64)) {
        if !self.look_around {
            return;
        }
        self.mouse_delta.0 += delta.0;
        self.mouse_delta.1 += delta.1;
    }

    pub fn update_eye(&mut self, eye: &mut CameraEye, delta_time: f32) {
        use InnerSpace;

        let mut local_move_direction: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);

        if self.direction_inputs[0] {
            local_move_direction.z -= 1.0;
        }
        if self.direction_inputs[1] {
            local_move_direction.z += 1.0;
        }
        if self.direction_inputs[2] {
            local_move_direction.x -= 1.0;
        }
        if self.direction_inputs[3] {
            local_move_direction.x += 1.0;
        }

        if local_move_direction.magnitude2() > 0.0 {
            local_move_direction = local_move_direction.normalize();
            let movement = eye.orientation * local_move_direction * self.speed * delta_time;
            eye.position += movement;
        }

        let mouse_sensitivity = 1.0 / 1000.0;
        let delta_yaw = Rad((self.mouse_delta.0 as f32) * mouse_sensitivity);
        let delta_pitch = Rad((self.mouse_delta.1 as f32) * mouse_sensitivity);

        self.mouse_delta = (0.0, 0.0);

        let yaw_rotation = Quaternion::from_angle_y(-delta_yaw);
        let pitch_rotation = Quaternion::from_angle_x(-delta_pitch);

        eye.orientation = (yaw_rotation * eye.orientation) * pitch_rotation;
        eye.orientation = eye.orientation.normalize();
    }
}
