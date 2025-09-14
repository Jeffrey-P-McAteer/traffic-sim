use crate::simulation::{SimulationState, PerformanceMetrics};
use crate::graphics::Viewport;
use anyhow::Result;

pub struct UiRenderer {
    // For now, we'll use a simple text-based overlay
    // In a full implementation, you might use egui or similar
}

impl UiRenderer {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    pub fn render(
        &mut self, 
        performance: &PerformanceMetrics, 
        state: &SimulationState,
        viewport: &Viewport
    ) -> Result<()> {
        // For now, we'll just log the performance metrics
        // In a full implementation, you would render this as overlay text
        
        let fps = if !performance.frame_time.is_zero() {
            1.0 / performance.frame_time.as_secs_f32()
        } else {
            0.0
        };
        
        log::debug!("=== Performance Metrics ===");
        log::debug!("FPS: {:.1}", fps);
        log::debug!("Frame Time: {:.2}ms", performance.frame_time.as_millis());
        log::debug!("Simulation Time: {:.2}ms", performance.simulation_time.as_millis());
        log::debug!("Render Time: {:.2}ms", performance.render_time.as_millis());
        
        log::debug!("=== Simulation State ===");
        log::debug!("Active Cars: {}", state.active_cars);
        log::debug!("Total Spawned: {}", state.total_spawned);
        log::debug!("Simulation Time: {:.1}s", state.time);
        
        log::debug!("=== Viewport ===");
        log::debug!("Position: ({:.1}, {:.1})", viewport.get_position().x, viewport.get_position().y);
        log::debug!("Zoom: {:.2}x", viewport.get_zoom());
        
        Ok(())
    }
}

// Simple text overlay data structure for future GUI implementation
#[allow(dead_code)]
pub struct TextOverlay {
    pub text: String,
    pub position: (f32, f32),
    pub color: [f32; 3],
    pub size: f32,
}

impl TextOverlay {
    pub fn new(text: String, x: f32, y: f32) -> Self {
        Self {
            text,
            position: (x, y),
            color: [1.0, 1.0, 1.0], // White
            size: 16.0,
        }
    }
}

// Helper function to create performance overlay text
pub fn create_performance_overlay(
    performance: &PerformanceMetrics,
    state: &SimulationState,
    viewport: &Viewport,
) -> Vec<TextOverlay> {
    let fps = if !performance.frame_time.is_zero() {
        1.0 / performance.frame_time.as_secs_f32()
    } else {
        0.0
    };
    
    vec![
        TextOverlay::new(format!("FPS: {:.1}", fps), 10.0, 10.0),
        TextOverlay::new(
            format!("Frame: {:.1}ms", performance.frame_time.as_millis()),
            10.0,
            30.0,
        ),
        TextOverlay::new(
            format!("Sim: {:.1}ms", performance.simulation_time.as_millis()),
            10.0,
            50.0,
        ),
        TextOverlay::new(
            format!("Render: {:.1}ms", performance.render_time.as_millis()),
            10.0,
            70.0,
        ),
        TextOverlay::new(
            format!("Cars: {}/{}", state.active_cars, state.total_spawned),
            10.0,
            110.0,
        ),
        TextOverlay::new(
            format!("Time: {:.1}s", state.time),
            10.0,
            130.0,
        ),
        TextOverlay::new(
            format!("Zoom: {:.2}x", viewport.get_zoom()),
            10.0,
            170.0,
        ),
        TextOverlay::new(
            format!("Pos: ({:.0}, {:.0})", viewport.get_position().x, viewport.get_position().y),
            10.0,
            190.0,
        ),
    ]
}

// Control hints for the user
pub fn create_control_hints() -> Vec<TextOverlay> {
    vec![
        TextOverlay::new("=== CONTROLS ===".to_string(), 10.0, 250.0),
        TextOverlay::new("Mouse: Drag to pan".to_string(), 10.0, 270.0),
        TextOverlay::new("Wheel: Zoom in/out".to_string(), 10.0, 290.0),
        TextOverlay::new("WASD/Arrows: Move camera".to_string(), 10.0, 310.0),
        TextOverlay::new("Home: Reset view".to_string(), 10.0, 330.0),
        TextOverlay::new("+/-: Zoom in/out".to_string(), 10.0, 350.0),
        TextOverlay::new("Space: Pause/Resume".to_string(), 10.0, 370.0),
    ]
}