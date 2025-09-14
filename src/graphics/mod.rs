use anyhow::Result;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::Window,
};
use crate::simulation::{SimulationState, PerformanceMetrics};

pub mod renderer;
pub mod viewport;
pub mod ui;

pub use renderer::*;
pub use viewport::*;
pub use ui::*;

pub struct GraphicsSystem {
    pub window: Window,
    pub renderer: TrafficRenderer,
    pub viewport: Viewport,
    pub ui: UiRenderer,
}

impl GraphicsSystem {
    pub async fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let window = match event_loop.create_window(
            winit::window::Window::default_attributes()
                .with_title("Traffic Simulator")
                .with_inner_size(winit::dpi::LogicalSize::new(1200, 800))
        ) {
            Ok(window) => window,
            Err(_) => {
                // Fallback for older winit versions
                return Err(anyhow::anyhow!("Failed to create window"));
            }
        };
        
        let renderer = TrafficRenderer::new(&window).await?;
        let viewport = Viewport::new(1200.0, 800.0);
        let ui = UiRenderer::new()?;
        
        Ok(Self {
            window,
            renderer,
            viewport,
            ui,
        })
    }
    
    pub fn handle_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                self.viewport.handle_mouse_input(*state, *button);
                true
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.viewport.handle_mouse_wheel(delta);
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.viewport.handle_mouse_move(position.x as f32, position.y as f32);
                true
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.viewport.handle_keyboard_input(event);
                true
            }
            WindowEvent::Resized(physical_size) => {
                self.renderer.resize(*physical_size);
                self.viewport.resize(physical_size.width as f32, physical_size.height as f32);
                true
            }
            WindowEvent::ScaleFactorChanged { inner_size_writer: _, .. } => {
                let size = self.renderer.size;
                self.renderer.resize(size);
                self.viewport.resize(size.width as f32, size.height as f32);
                true
            }
            _ => false,
        }
    }
    
    pub fn render(&mut self, state: &SimulationState, performance: &PerformanceMetrics) -> Result<()> {
        // Update viewport
        self.viewport.update();
        
        // Render the scene
        let view_matrix = self.viewport.get_view_matrix();
        self.renderer.render(state, &view_matrix)?;
        
        // Render UI overlay
        self.ui.render(performance, state, &self.viewport)?;
        
        Ok(())
    }
}