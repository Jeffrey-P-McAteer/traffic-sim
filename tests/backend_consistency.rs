use traffic_sim::{
    config::{SimulationConfig, CarsConfig, RouteConfig},
    simulation::SimulationState,
    compute::{ComputeBackend, SimulationBackend},
};
use anyhow::Result;

/// Test that CPU and GPU backends produce identical results with the same seed
#[test]
fn test_cpu_gpu_consistency() -> Result<()> {
    // Load test configuration
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")?;
    
    let seed = Some(12345u64); // Fixed seed for reproducibility
    let test_duration = 5.0; // Test first 5 seconds
    let tolerance = 0.01; // 1% tolerance for position differences
    
    // Create CPU backend
    let mut cpu_backend = ComputeBackend::new_cpu(
        config.cars.clone(), 
        config.route.clone(), 
        seed
    );
    
    // Create GPU backend (skip if no GPU available)
    let mut gpu_backend = match ComputeBackend::new_gpu(
        config.cars.clone(), 
        config.route.clone(), 
        seed
    ) {
        Ok(backend) => backend,
        Err(e) => {
            println!("Skipping GPU test: {}", e);
            return Ok(());
        }
    };
    
    // Initialize simulation states
    let dt = 1.0 / 60.0; // 60 FPS
    let mut cpu_state = SimulationState::new(dt);
    let mut gpu_state = SimulationState::new(dt);
    
    // Record states for comparison
    let mut cpu_snapshots = Vec::new();
    let mut gpu_snapshots = Vec::new();
    
    let steps = (test_duration / dt) as usize;
    
    for step in 0..steps {
        // Update both backends
        cpu_backend.update(&mut cpu_state)?;
        gpu_backend.update(&mut gpu_state)?;
        
        // Take snapshots every 60 steps (1 second intervals)
        if step % 60 == 0 {
            cpu_snapshots.push(cpu_state.clone());
            gpu_snapshots.push(gpu_state.clone());
        }
    }
    
    // Compare results
    assert_eq!(cpu_snapshots.len(), gpu_snapshots.len(), "Snapshot count mismatch");
    
    for (i, (cpu_snap, gpu_snap)) in cpu_snapshots.iter().zip(gpu_snapshots.iter()).enumerate() {
        println!("Comparing snapshot {} at t={:.1}s", i, i as f32);
        
        // Compare car counts
        assert_eq!(cpu_snap.cars.len(), gpu_snap.cars.len(), 
                   "Car count mismatch at snapshot {}: CPU={}, GPU={}", 
                   i, cpu_snap.cars.len(), gpu_snap.cars.len());
        
        assert_eq!(cpu_snap.total_spawned, gpu_snap.total_spawned,
                   "Total spawned mismatch at snapshot {}: CPU={}, GPU={}",
                   i, cpu_snap.total_spawned, gpu_snap.total_spawned);
        
        // Compare individual car positions
        for (cpu_car, gpu_car) in cpu_snap.cars.iter().zip(gpu_snap.cars.iter()) {
            let pos_diff_x = (cpu_car.position.x - gpu_car.position.x).abs();
            let pos_diff_y = (cpu_car.position.y - gpu_car.position.y).abs();
            
            let max_pos = cpu_car.position.x.abs().max(cpu_car.position.y.abs()).max(1.0);
            let relative_error_x = pos_diff_x / max_pos;
            let relative_error_y = pos_diff_y / max_pos;
            
            assert!(relative_error_x < tolerance, 
                    "Car {} X position differs too much at snapshot {}: CPU={:.3}, GPU={:.3}, error={:.3}% (tolerance={:.1}%)",
                    cpu_car.id.0, i, cpu_car.position.x, gpu_car.position.x, relative_error_x * 100.0, tolerance * 100.0);
            
            assert!(relative_error_y < tolerance,
                    "Car {} Y position differs too much at snapshot {}: CPU={:.3}, GPU={:.3}, error={:.3}% (tolerance={:.1}%)", 
                    cpu_car.id.0, i, cpu_car.position.y, gpu_car.position.y, relative_error_y * 100.0, tolerance * 100.0);
            
            // Compare velocities
            let vel_diff_x = (cpu_car.velocity.x - gpu_car.velocity.x).abs();
            let vel_diff_y = (cpu_car.velocity.y - gpu_car.velocity.y).abs();
            
            let max_vel = cpu_car.velocity.x.abs().max(cpu_car.velocity.y.abs()).max(1.0);
            let vel_error_x = vel_diff_x / max_vel;
            let vel_error_y = vel_diff_y / max_vel;
            
            assert!(vel_error_x < tolerance,
                    "Car {} X velocity differs too much at snapshot {}: CPU={:.3}, GPU={:.3}, error={:.3}%",
                    cpu_car.id.0, i, cpu_car.velocity.x, gpu_car.velocity.x, vel_error_x * 100.0);
                    
            assert!(vel_error_y < tolerance,
                    "Car {} Y velocity differs too much at snapshot {}: CPU={:.3}, GPU={:.3}, error={:.3}%",
                    cpu_car.id.0, i, cpu_car.velocity.y, gpu_car.velocity.y, vel_error_y * 100.0);
        }
        
        println!("✓ Snapshot {} passed with {} cars", i, cpu_snap.cars.len());
    }
    
    println!("✓ All CPU/GPU consistency tests passed!");
    Ok(())
}

/// Test that both backends produce identical spawning with same seed
#[test] 
fn test_spawn_consistency() -> Result<()> {
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")?;
    let seed = Some(54321u64);
    
    let mut cpu_backend = ComputeBackend::new_cpu(config.cars.clone(), config.route.clone(), seed);
    
    let mut gpu_backend = match ComputeBackend::new_gpu(config.cars.clone(), config.route.clone(), seed) {
        Ok(backend) => backend,
        Err(_) => return Ok(()), // Skip if no GPU
    };
    
    let dt = 1.0 / 60.0;
    let mut cpu_state = SimulationState::new(dt);
    let mut gpu_state = SimulationState::new(dt);
    
    // Run for 3 seconds to spawn some cars
    for _ in 0..(3.0 / dt) as usize {
        cpu_backend.update(&mut cpu_state)?;
        gpu_backend.update(&mut gpu_state)?;
    }
    
    // Should have spawned the same number of cars
    assert_eq!(cpu_state.total_spawned, gpu_state.total_spawned,
               "Different spawn counts: CPU={}, GPU={}", 
               cpu_state.total_spawned, gpu_state.total_spawned);
    
    assert_eq!(cpu_state.cars.len(), gpu_state.cars.len(),
               "Different active car counts: CPU={}, GPU={}",
               cpu_state.cars.len(), gpu_state.cars.len());
    
    println!("✓ Spawn consistency test passed: {} cars spawned", cpu_state.total_spawned);
    Ok(())
}