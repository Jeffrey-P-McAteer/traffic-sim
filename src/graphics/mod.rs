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
    pub window: std::sync::Arc<Window>,
    pub renderer: TrafficRenderer,
    pub viewport: Viewport,
    pub ui: UiRenderer,
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl GraphicsSystem {
    pub async fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let window = std::sync::Arc::new(
            winit::window::WindowBuilder::new()
                .with_title("Traffic Simulator")
                .with_inner_size(winit::dpi::LogicalSize::new(1200, 800))
                .build(event_loop)?
        );
        
        let renderer = TrafficRenderer::new(window.clone()).await?;
        let viewport = Viewport::new(1200.0, 800.0);
        let ui = UiRenderer::new()?;
        
        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            renderer.device(),
            renderer.config().format,
            None,
            1,
        );
        
        Ok(Self {
            window,
            renderer,
            viewport,
            ui,
            egui_ctx,
            egui_winit,
            egui_renderer,
        })
    }
    
    pub fn handle_input(&mut self, event: &WindowEvent) -> bool {
        // Handle egui input first
        let response = self.egui_winit.on_window_event(&self.window, event);
        if response.consumed {
            return true;
        }
        
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
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = self.renderer.size;
                self.renderer.resize(size);
                self.viewport.resize(size.width as f32, size.height as f32);
                true
            }
            _ => false,
        }
    }
    
    pub fn render(
        &mut self, 
        state: &SimulationState, 
        performance: &PerformanceMetrics,
        paused: bool,
        simulation_speed: f32,
        frame_count: u64,
        route_file: &str,
        cars_file: &str,
        seed: Option<u64>,
        font_size: f32
    ) -> Result<()> {
        // Update viewport
        self.viewport.update();
        
        // Get current texture for rendering
        let output = self.renderer.surface().get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.renderer.device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            }
        );
        
        // Render the 3D scene first
        let view_matrix = self.viewport.get_view_matrix();
        self.renderer.render_to_texture(state, &view_matrix, &view, &mut encoder)?;
        
        // Prepare egui
        let raw_input = self.egui_winit.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Render UI overlay with egui
            self.ui.render_egui(ctx, performance, state, &self.viewport, paused, simulation_speed, frame_count, route_file, cars_file, seed, font_size);
        });
        
        self.egui_winit.handle_platform_output(&self.window, full_output.platform_output);
        
        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(self.renderer.device(), self.renderer.queue(), *id, image_delta);
        }
        
        self.egui_renderer.update_buffers(
            self.renderer.device(),
            self.renderer.queue(),
            &mut encoder,
            &tris,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.renderer.size.width, self.renderer.size.height],
                pixels_per_point: self.window.scale_factor() as f32,
            },
        );
        
        // Render egui on top of the scene
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear, we want to render on top
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            self.egui_renderer.render(&mut rpass, &tris, &egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.renderer.size.width, self.renderer.size.height],
                pixels_per_point: self.window.scale_factor() as f32,
            });
        }
        
        // Submit commands and present
        self.renderer.queue().submit(std::iter::once(encoder.finish()));
        output.present();
        
        // Cleanup textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        
        Ok(())
    }
}