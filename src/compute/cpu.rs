use crate::simulation::{SimulationState, PhysicsEngine, TrafficManager};
use crate::config::{CarsConfig, RouteConfig};
use anyhow::Result;
use super::SimulationBackend;

pub struct CpuBackend {
    physics_engine: PhysicsEngine,
    traffic_manager: TrafficManager,
}

impl CpuBackend {
    pub fn new(
        cars_config: CarsConfig, 
        route_config: RouteConfig,
        seed: Option<u64>
    ) -> Self {
        let physics_engine = PhysicsEngine::new(
            route_config.clone(), 
            cars_config.collision_avoidance.clone()
        );
        
        let traffic_manager = TrafficManager::new(
            cars_config,
            route_config,
            seed
        );
        
        Self {
            physics_engine,
            traffic_manager,
        }
    }
}

impl SimulationBackend for CpuBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()> {
        // Update traffic management (spawning/despawning, behavior)
        self.traffic_manager.update(state);
        
        // Update physics (movement, collision avoidance)
        self.physics_engine.update(state);
        
        Ok(())
    }
    
    fn get_name(&self) -> &'static str {
        "CPU"
    }
    
    fn supports_gpu(&self) -> bool {
        false
    }
}