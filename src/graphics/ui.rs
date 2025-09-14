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
        viewport: &Viewport,
        paused: bool,
        simulation_speed: f32,
        frame_count: u64,
        route_file: &str,
        cars_file: &str
    ) -> Result<()> {
        // For now, we'll just log the performance metrics
        // In a full implementation, you would render this as overlay text
        
        let fps = if !performance.frame_time.is_zero() {
            1.0 / performance.frame_time.as_secs_f32()
        } else {
            0.0
        };
        
        // Create text overlays for GUI rendering in lower-left corner
        let status = if paused { "PAUSED" } else { "RUNNING" };
        
        let overlays = vec![
            // Status section
            TextOverlay::new(format!("Status: {}", status), 15.0, 15.0),
            TextOverlay::new(format!("Cars: {}/{}", state.active_cars, state.total_spawned), 15.0, 35.0),
            TextOverlay::new(format!("Time: {:.1}s", state.time), 15.0, 55.0),
            TextOverlay::new(format!("Speed: {:.2}x", simulation_speed), 15.0, 75.0),
            TextOverlay::new(format!("FPS: {:.0}", fps), 15.0, 95.0),
            TextOverlay::new(format!("Frame: {}", frame_count), 15.0, 115.0),
            
            // Files section
            TextOverlay::new(format!("Route: {}", route_file), 15.0, 145.0),
            TextOverlay::new(format!("Cars: {}", cars_file), 15.0, 165.0),
            
            // Camera info
            TextOverlay::new(format!("Zoom: {:.2}x", viewport.get_zoom()), 15.0, 195.0),
            TextOverlay::new(format!("Pos: ({:.0}, {:.0})", 
                           viewport.get_position().x, viewport.get_position().y), 15.0, 215.0),
            
            // Controls help
            TextOverlay::new("=== CONTROLS ===".to_string(), 15.0, 245.0),
            TextOverlay::new("Mouse: Drag=pan, Wheel=zoom".to_string(), 15.0, 265.0),
            TextOverlay::new("WASD/Arrows: Move camera".to_string(), 15.0, 285.0),
            TextOverlay::new("Home: Reset view".to_string(), 15.0, 305.0),
            TextOverlay::new("Space: Pause/Resume".to_string(), 15.0, 325.0),
            TextOverlay::new("1-5: Speed (0.25x-4x)".to_string(), 15.0, 345.0),
            TextOverlay::new("R: Reset simulation".to_string(), 15.0, 365.0),
            TextOverlay::new("ESC: Exit".to_string(), 15.0, 385.0),
        ];
        
        // In a full implementation with actual text rendering:
        // - These overlays would be rendered as text on top of the 3D scene
        // - For now, we silently collect them and log debug info
        
        log::debug!("UI Overlays: {} text elements ready for rendering", overlays.len());
        log::debug!("Performance: FPS={:.1}, Frame={:.2}ms, Sim={:.2}ms", 
                   fps, performance.frame_time.as_millis(), performance.simulation_time.as_millis());
        
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
        TextOverlay::new("=== VIEWPORT CONTROLS ===".to_string(), 10.0, 250.0),
        TextOverlay::new("Mouse Drag: Pan view".to_string(), 10.0, 270.0),
        TextOverlay::new("Mouse Wheel: Zoom in/out".to_string(), 10.0, 290.0),
        TextOverlay::new("WASD/Arrow Keys: Move camera".to_string(), 10.0, 310.0),
        TextOverlay::new("Home: Reset view to center".to_string(), 10.0, 330.0),
        TextOverlay::new("+/- Keys: Zoom in/out".to_string(), 10.0, 350.0),
        TextOverlay::new("=== SIMULATION CONTROLS ===".to_string(), 10.0, 380.0),
        TextOverlay::new("Space: Pause/Resume".to_string(), 10.0, 400.0),
        TextOverlay::new("R: Reset simulation".to_string(), 10.0, 420.0),
        TextOverlay::new("1-5: Set speed (0.25x - 4x)".to_string(), 10.0, 440.0),
        TextOverlay::new("ESC: Exit simulation".to_string(), 10.0, 460.0),
    ]
}