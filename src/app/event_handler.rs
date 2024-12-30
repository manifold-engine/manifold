use super::ManifoldApp;

use super::window_management::WindowManager;

use winit::event::*;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

use log::{debug, warn};
use winit::window::CursorIcon;

pub trait EventHandler {
    fn handle_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent);
    fn handle_keyboard_input(&mut self, event_loop: &ActiveEventLoop, key_event: KeyEvent);
}

impl EventHandler for ManifoldApp {
    fn handle_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        debug!("{event:?}");
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                self.window.as_mut().unwrap().set_cursor_visible(focused);
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size.width, physical_size.height);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scaled_width = (self.size.width as f64 * scale_factor).floor() as u32;
                let scaled_height = (self.size.height as f64 * scale_factor).floor() as u32;
                self.resize(scaled_width, scaled_height);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event_loop, event);
            }
            WindowEvent::CursorEntered { .. } | WindowEvent::CursorLeft { .. } => {
                self.cursor_position = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(prev) = self.cursor_position {
                    let delta = (position.x - prev.0, position.y - prev.1);
                    self.renderer.as_mut().unwrap().handle_mouse_delta(delta);
                }
                self.cursor_position = Some((position.x, position.y));
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
                self.renderer.as_mut().unwrap().render();
            }
            _ => (),
        }
    }

    fn handle_keyboard_input(&mut self, event_loop: &ActiveEventLoop, key_event: KeyEvent) {
        match key_event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => {
                if key_event.state == ElementState::Pressed {
                    warn!("Escape pressed, exiting the application.");
                    event_loop.exit();
                }
            }
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                let icon = if key_event.state == ElementState::Pressed {
                    CursorIcon::Grab
                } else {
                    CursorIcon::Pointer
                };
                self.window.as_ref().unwrap().set_cursor(icon);
                self.renderer
                    .as_mut()
                    .unwrap()
                    .handle_camera_movement(key_event);
            }
            PhysicalKey::Code(KeyCode::KeyW)
            | PhysicalKey::Code(KeyCode::KeyS)
            | PhysicalKey::Code(KeyCode::KeyA)
            | PhysicalKey::Code(KeyCode::KeyD) => {
                self.renderer
                    .as_mut()
                    .unwrap()
                    .handle_camera_movement(key_event);
            }
            PhysicalKey::Code(KeyCode::KeyJ) | PhysicalKey::Code(KeyCode::KeyK) => {
                self.renderer
                    .as_mut()
                    .unwrap()
                    .handle_slice_movement(key_event);
            }
            _ => (),
        }
    }
}
