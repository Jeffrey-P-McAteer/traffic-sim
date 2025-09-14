use super::{Car, SimulationState, BehaviorState};
use crate::config::{DriverBehavior, CarsConfig, RouteConfig};
use rand::{Rng, SeedableRng};
use rand_distr::{Normal, Distribution};
use rand::rngs::StdRng;

#[derive(Debug, Clone)]
struct BehaviorUpdate {
    target_speed: f32,
    target_lane: Option<u32>,
    lane_change_requested: bool,
}

pub struct BehaviorEngine {
    behaviors: Vec<(String, DriverBehavior)>,
    route: RouteConfig,
    rng: StdRng,
}

impl BehaviorEngine {
    pub fn new(cars_config: &CarsConfig, route: RouteConfig, seed: Option<u64>) -> Self {
        let behaviors: Vec<(String, DriverBehavior)> = cars_config.behavior
            .iter()
            .map(|(name, behavior)| (name.clone(), behavior.clone()))
            .collect();
        
        let rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_entropy()
        };
        
        Self {
            behaviors,
            route,
            rng,
        }
    }
    
    pub fn update(&mut self, state: &mut SimulationState) {
        let mut updates = Vec::new();
        
        // Collect behavior updates
        for (i, car) in state.cars.iter().enumerate() {
            let update = self.calculate_car_behavior_update(car, state);
            updates.push((i, update));
        }
        
        // Apply updates
        for (i, update) in updates {
            if let Some(car) = state.cars.get_mut(i) {
                car.behavior.target_speed = update.target_speed;
                car.target_lane = update.target_lane;
                if update.lane_change_requested {
                    car.behavior.last_lane_change_time = state.time;
                    car.lane_change_progress = 0.0;
                }
            }
        }
    }
    
    fn calculate_car_behavior_update(&mut self, car: &Car, state: &SimulationState) -> BehaviorUpdate {
        let mut update = BehaviorUpdate {
            target_speed: self.calculate_target_speed(car),
            target_lane: car.target_lane,
            lane_change_requested: false,
        };
        
        // Check for lane change decisions
        if let Some(new_target_lane) = self.check_lane_change_decision(car, state) {
            update.target_lane = Some(new_target_lane);
            update.lane_change_requested = true;
        }
        
        // Check for exit decisions
        self.check_exit_decision_for_car(car, state);
        
        update
    }
    
    fn calculate_target_speed(&mut self, car: &Car) -> f32 {
        let base_speed = car.preferred_speed;
        let variance = car.behavior.speed_variance;
        
        // Add some randomness to speed preference
        let speed_noise = if variance != 1.0 {
            let normal = Normal::new(1.0, (variance - 1.0).abs() * 0.1).unwrap();
            normal.sample(&mut self.rng)
        } else {
            1.0
        };
        
        // Apply speed limits
        let speed_limit = self.route.route.traffic_rules.speed_limit;
        let min_speed = self.route.route.traffic_rules.min_speed;
        
        (base_speed * variance * speed_noise)
            .max(min_speed)
            .min(speed_limit)
    }
    
    fn check_lane_change_decision(&mut self, car: &Car, state: &SimulationState) -> Option<u32> {
        // Don't change lanes if already changing
        if car.target_lane.is_some() {
            return None;
        }
        
        // Check if enough time has passed since last lane change
        let time_since_change = state.time - car.behavior.last_lane_change_time;
        let min_change_interval = 60.0 / car.behavior.lane_change_frequency; // Convert from per-minute to seconds
        
        if time_since_change < min_change_interval {
            return None;
        }
        
        let route_geom = &self.route.route.geometry;
        let total_lanes = route_geom.lane_count;
        
        // Determine possible lane changes
        let can_change_left = car.current_lane > 1;
        let can_change_right = car.current_lane < total_lanes;
        
        if !can_change_left && !can_change_right {
            return None;
        }
        
        // Calculate lane change probability based on behavior
        let base_probability = car.behavior.lane_change_frequency / 60.0; // per second
        let lane_change_chance = base_probability * state.dt;
        
        if self.rng.gen::<f32>() < lane_change_chance {
            // Decide which lane to change to
            let target_lane = if can_change_left && can_change_right {
                if self.rng.gen_bool(0.5) {
                    car.current_lane - 1
                } else {
                    car.current_lane + 1
                }
            } else if can_change_left {
                car.current_lane - 1
            } else {
                car.current_lane + 1
            };
            
            // Check if lane change is safe
            if self.is_lane_change_safe(car, target_lane, state) {
                return Some(target_lane);
            }
        }
        
        None
    }
    
    fn is_lane_change_safe(&self, car: &Car, target_lane: u32, state: &SimulationState) -> bool {
        let route_geom = &self.route.route.geometry;
        let center = nalgebra::Point2::new(route_geom.center_x, route_geom.center_y);
        let to_car = car.position - center;
        let car_angle = to_car.y.atan2(to_car.x);
        
        let safety_distance = car.length + 10.0; // Minimum safe distance
        
        for other_car in &state.cars {
            if other_car.id == car.id || other_car.current_lane != target_lane {
                continue;
            }
            
            let to_other = other_car.position - center;
            let other_angle = to_other.y.atan2(to_other.x);
            
            // Calculate angular distance
            let mut angle_diff = (other_angle - car_angle).abs();
            if angle_diff > std::f32::consts::PI {
                angle_diff = 2.0 * std::f32::consts::PI - angle_diff;
            }
            
            let arc_distance = angle_diff * to_car.magnitude();
            
            if arc_distance < safety_distance {
                return false;
            }
        }
        
        true
    }
    
    fn check_exit_decision_for_car(&mut self, car: &Car, _state: &SimulationState) {
        // Find nearby exits
        let route_geom = &self.route.route.geometry;
        let center = nalgebra::Point2::new(route_geom.center_x, route_geom.center_y);
        let to_car = car.position - center;
        let car_angle = to_car.y.atan2(to_car.x).to_degrees();
        
        // Normalize angle to 0-360 range
        let car_angle = if car_angle < 0.0 {
            car_angle + 360.0
        } else {
            car_angle
        };
        
        for exit in &self.route.route.exits {
            // Check if exit is nearby (within 30 degrees)
            let angle_diff = (exit.angle - car_angle).abs();
            let angle_diff = if angle_diff > 180.0 {
                360.0 - angle_diff
            } else {
                angle_diff
            };
            
            if angle_diff < 30.0 && car.current_lane == exit.lane {
                // Decide whether to take the exit
                if self.rng.gen::<f32>() < car.behavior.exit_probability {
                    // TODO: Implement exit logic
                    // For now, we'll just mark the car for removal
                    // This would be handled by the traffic manager
                }
            }
        }
    }
    
    pub fn create_behavior_state(&mut self, behavior_name: &str) -> BehaviorState {
        // Find the behavior configuration
        let behavior = self.behaviors
            .iter()
            .find(|(name, _)| name == behavior_name)
            .map(|(_, behavior)| behavior.clone())
            .unwrap_or_else(|| {
                // Default to "normal" behavior if not found
                self.behaviors
                    .iter()
                    .find(|(name, _)| name == "normal")
                    .map(|(_, behavior)| behavior.clone())
                    .unwrap_or(DriverBehavior {
                        name: "default".to_string(),
                        weight: 100,
                        following_distance_factor: 1.0,
                        lane_change_frequency: 0.8,
                        speed_variance: 1.0,
                        reaction_time: 1.2,
                        exit_probability: 0.25,
                    })
            });
        
        BehaviorState {
            following_distance_factor: behavior.following_distance_factor,
            lane_change_frequency: behavior.lane_change_frequency,
            speed_variance: behavior.speed_variance,
            reaction_time: behavior.reaction_time,
            exit_probability: behavior.exit_probability,
            last_lane_change_time: 0.0,
            target_speed: 25.0, // Will be updated by physics
        }
    }
    
    pub fn select_random_behavior(&mut self) -> String {
        let total_weight: u32 = self.behaviors.iter().map(|(_, b)| b.weight).sum();
        let mut random_value = self.rng.gen_range(0..total_weight);
        
        for (name, behavior) in &self.behaviors {
            if random_value < behavior.weight {
                return name.clone();
            }
            random_value -= behavior.weight;
        }
        
        // Fallback to first behavior
        self.behaviors.first()
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "normal".to_string())
    }
}