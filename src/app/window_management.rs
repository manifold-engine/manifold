use super::ManifoldApp;

use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub trait WindowManager {
    fn setup_window(&mut self, event_loop: &ActiveEventLoop);
    fn resize(&mut self, width: u32, height: u32);
}

impl WindowManager for ManifoldApp {
    fn setup_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        };

        let window_attributes = Window::default_attributes().with_title("Manifold");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            if let (Some(surface), Some(device), Some(config)) =
                (&self.surface, &self.device, &mut self.config)
            {
                config.width = width;
                config.height = height;
                surface.configure(device, config);
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}