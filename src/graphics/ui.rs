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
        
        // In a real UI implementation, this would render text overlays on the screen
        // For now, we output to console with comprehensive status information
        
        let status = if paused { "PAUSED" } else { "RUNNING" };
        
        println!("\n╔═══════════════════════════════════════════════════════════════╗");
        println!("║                    TRAFFIC SIMULATION STATUS                  ║");
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║ Status: {:8} │ Frame: {:10} │ Speed: {:6.2}x      ║", status, frame_count, simulation_speed);
        println!("║ Cars:   {:3}/{:<8} │ Time:  {:10.1}s │ FPS:   {:6.1}      ║", 
                 state.active_cars, state.total_spawned, state.time, fps);
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║ Route File: {:<49} ║", route_file);
        println!("║ Cars File:  {:<49} ║", cars_file);
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║                       VIEWPORT CONTROLS                       ║");
        println!("║ Current Position: ({:6.0}, {:6.0}) │ Zoom: {:6.2}x            ║", 
                 viewport.get_position().x, viewport.get_position().y, viewport.get_zoom());
        println!("║ Mouse Drag: Pan view        │ Mouse Wheel: Zoom in/out       ║");
        println!("║ WASD/Arrow Keys: Move camera │ +/- Keys: Zoom in/out         ║");
        println!("║ Home: Reset view to center  │                                ║");
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║                      SIMULATION CONTROLS                     ║");
        println!("║ SPACE: Pause/Resume         │ R: Reset simulation            ║");
        println!("║ 1-5: Set speed (0.25x-4x)  │ F1: Toggle performance display ║");
        println!("║ ESC: Exit simulation        │                                ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        
        // Also log debug info for development
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