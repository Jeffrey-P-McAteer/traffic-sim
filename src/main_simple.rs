use anyhow::Result;
use log::info;
use std::time::{Duration, Instant};

use traffic_sim::{
    config::SimulationConfig,
    simulation::{SimulationState, PerformanceTracker},
    compute::{ComputeBackend, SimulationBackend},
};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    info!("Starting Traffic Simulator (Console Mode)");
    
    // Load configuration
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")?;
    info!("Loaded configuration: {} cars max, route: {}", 
          config.cars.simulation.total_cars, 
          config.route.route.name);
    
    // Initialize simulation state
    let dt = 1.0 / 60.0; // 60 FPS simulation timestep
    let mut simulation_state = SimulationState::new(dt);
    
    // Initialize CPU compute backend
    let mut compute_backend = ComputeBackend::new_cpu(
        config.cars.clone(),
        config.route.clone(),
        config.cars.random.seed
    );
    
    info!("Compute backend: {}", compute_backend.get_name());
    
    // Initialize performance tracker
    let mut performance_tracker = PerformanceTracker::new(
        config.cars.performance.timing_samples as usize
    );
    
    // Run simulation for a specified duration
    let simulation_duration = Duration::from_secs(5); // 5 seconds
    let start_time = Instant::now();
    let mut last_update = Instant::now();
    let mut frame_count = 0;
    
    info!("Running simulation for {} seconds...", simulation_duration.as_secs());
    
    while start_time.elapsed() < simulation_duration {
        performance_tracker.start_frame();
        performance_tracker.start_simulation();
        
        // Update simulation
        compute_backend.update(&mut simulation_state)?;
        
        performance_tracker.end_simulation();
        performance_tracker.end_frame();
        
        frame_count += 1;
        
        // Print status every second
        if last_update.elapsed() >= Duration::from_secs(1) {
            let avg_frame_time = performance_tracker.average_frame_time();
            let avg_sim_time = performance_tracker.average_simulation_time();
            let fps = performance_tracker.fps();
            
            info!("Frame {}: {} cars active, {:.1} FPS, Frame: {:.2}ms, Sim: {:.2}ms", 
                  frame_count,
                  simulation_state.active_cars,
                  fps,
                  avg_frame_time.as_millis(),
                  avg_sim_time.as_millis()
            );
            
            last_update = Instant::now();
        }
        
        // Sleep to maintain target framerate
        let target_frame_time = Duration::from_secs_f64(dt as f64);
        let elapsed = performance_tracker.average_frame_time();
        
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
    }
    
    // Final statistics
    let total_time = start_time.elapsed();
    let avg_fps = frame_count as f64 / total_time.as_secs_f64();
    
    info!("Simulation completed!");
    info!("Total time: {:.2}s", total_time.as_secs_f64());
    info!("Total frames: {}", frame_count);
    info!("Average FPS: {:.1}", avg_fps);
    info!("Final car count: {} active, {} total spawned", 
          simulation_state.active_cars, 
          simulation_state.total_spawned);
    
    Ok(())
}