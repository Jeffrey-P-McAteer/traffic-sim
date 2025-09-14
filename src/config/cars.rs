use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use super::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CarsConfig {
    pub simulation: SimulationParams,
    pub car_types: Vec<CarType>,
    pub behavior: HashMap<String, DriverBehavior>,
    pub collision_avoidance: CollisionAvoidance,
    pub traffic_flow: TrafficFlow,
    pub random: RandomConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimulationParams {
    pub total_cars: u32,
    pub spawn_rate: f32,
    pub simulation_duration: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CarType {
    pub id: String,
    pub weight: u32,
    pub length: f32,
    pub width: f32,
    pub max_acceleration: f32,
    pub max_deceleration: f32,
    pub preferred_speed: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DriverBehavior {
    pub name: String,
    pub weight: u32,
    pub following_distance_factor: f32,
    pub lane_change_frequency: f32,
    pub speed_variance: f32,
    pub reaction_time: f32,
    pub exit_probability: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollisionAvoidance {
    pub safety_margin: f32,
    pub emergency_brake_distance: f32,
    pub warning_distance: f32,
    pub lateral_safety_margin: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficFlow {
    pub entry_intervals: Vec<EntryInterval>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntryInterval {
    pub entry_id: String,
    pub min_interval: f32,
    pub max_interval: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RandomConfig {
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub enable_gpu_timing: bool,
    pub enable_cpu_timing: bool,
    pub timing_samples: u32,
}

impl Validate for CarsConfig {
    fn validate(&self) -> Result<()> {
        // Validate simulation parameters
        let sim = &self.simulation;
        if sim.total_cars == 0 {
            return Err(anyhow!("Total cars must be greater than zero"));
        }
        
        if sim.spawn_rate <= 0.0 {
            return Err(anyhow!("Spawn rate must be positive"));
        }
        
        if sim.simulation_duration <= 0.0 {
            return Err(anyhow!("Simulation duration must be positive"));
        }
        
        // Validate car types
        if self.car_types.is_empty() {
            return Err(anyhow!("At least one car type must be defined"));
        }
        
        let total_car_weight: u32 = self.car_types.iter().map(|ct| ct.weight).sum();
        if total_car_weight != 100 {
            return Err(anyhow!("Car type weights must sum to 100, got {}", total_car_weight));
        }
        
        for car_type in &self.car_types {
            if car_type.length <= 0.0 || car_type.width <= 0.0 {
                return Err(anyhow!("Car dimensions must be positive"));
            }
            
            if car_type.max_acceleration <= 0.0 || car_type.max_deceleration <= 0.0 {
                return Err(anyhow!("Car acceleration values must be positive"));
            }
            
            if car_type.preferred_speed <= 0.0 {
                return Err(anyhow!("Preferred speed must be positive"));
            }
        }
        
        // Validate behaviors
        if self.behavior.is_empty() {
            return Err(anyhow!("At least one behavior must be defined"));
        }
        
        let total_behavior_weight: u32 = self.behavior.values().map(|b| b.weight).sum();
        if total_behavior_weight != 100 {
            return Err(anyhow!("Behavior weights must sum to 100, got {}", total_behavior_weight));
        }
        
        for (name, behavior) in &self.behavior {
            if behavior.following_distance_factor <= 0.0 {
                return Err(anyhow!("Following distance factor for '{}' must be positive", name));
            }
            
            if behavior.lane_change_frequency < 0.0 {
                return Err(anyhow!("Lane change frequency for '{}' must be non-negative", name));
            }
            
            if behavior.speed_variance <= 0.0 {
                return Err(anyhow!("Speed variance for '{}' must be positive", name));
            }
            
            if behavior.reaction_time <= 0.0 {
                return Err(anyhow!("Reaction time for '{}' must be positive", name));
            }
            
            if behavior.exit_probability < 0.0 || behavior.exit_probability > 1.0 {
                return Err(anyhow!("Exit probability for '{}' must be in range [0, 1]", name));
            }
        }
        
        // Validate collision avoidance
        let collision = &self.collision_avoidance;
        if collision.safety_margin < 0.0 {
            return Err(anyhow!("Safety margin must be non-negative"));
        }
        
        if collision.emergency_brake_distance <= 0.0 || collision.warning_distance <= 0.0 {
            return Err(anyhow!("Brake distances must be positive"));
        }
        
        if collision.emergency_brake_distance >= collision.warning_distance {
            return Err(anyhow!("Emergency brake distance must be less than warning distance"));
        }
        
        // Validate performance config
        let perf = &self.performance;
        if perf.timing_samples == 0 {
            return Err(anyhow!("Timing samples must be greater than zero"));
        }
        
        Ok(())
    }
}