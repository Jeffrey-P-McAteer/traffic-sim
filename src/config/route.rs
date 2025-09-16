use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use super::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    pub route: Route,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    pub name: String,
    pub description: String,
    pub geometry: RouteGeometry,
    pub entries: Vec<EntryPoint>,
    pub exits: Vec<ExitPoint>,
    pub traffic_rules: TrafficRules,
    pub surface: RoadSurface,
    #[serde(default)]
    pub signals: TrafficSignals,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteGeometry {
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub center_x: f32,
    pub center_y: f32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub lane_width: f32,
    pub lane_count: u32,
    // Cloverleaf-specific fields
    #[serde(default)]
    pub highway_width: Option<f32>,
    #[serde(default)]
    pub highway_length: Option<f32>,
    #[serde(default)]
    pub loop_radius: Option<f32>,
    #[serde(default)]
    pub ramp_width: Option<f32>,
    #[serde(default)]
    pub ramp_lanes: Option<u32>,
    // Grid-specific fields
    #[serde(default)]
    pub grid: Option<Vec<Vec<String>>>,
    #[serde(default)]
    pub cell_size: Option<f32>,
    #[serde(default)]
    pub spawn_points: Option<Vec<GridPoint>>,
    #[serde(default)]
    pub exit_points: Option<Vec<GridPoint>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridPoint {
    pub id: String,
    pub row: usize,
    pub col: usize,
    pub weight: Option<f32>, // probability weight for random selection
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntryPoint {
    pub id: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub angle: f32,
    pub position: String,
    pub lane: u32,
    pub merge_distance: f32,
    // Cloverleaf-specific fields
    #[serde(default)]
    pub loop_entry_angle: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExitPoint {
    pub id: String,
    #[serde(rename = "type")]
    pub exit_type: String,
    pub angle: f32,
    pub position: String,
    pub lane: u32,
    pub exit_distance: f32,
    // Cloverleaf-specific fields
    #[serde(default)]
    pub loop_exit_angle: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficRules {
    pub speed_limit: f32,
    pub min_speed: f32,
    pub following_distance: f32,
    pub lane_change_time: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoadSurface {
    pub friction_coefficient: f32,
    pub banking_angle: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TrafficSignals {}

impl Validate for RouteConfig {
    fn validate(&self) -> Result<()> {
        let geometry = &self.route.geometry;
        
        if geometry.geometry_type != "donut" && geometry.geometry_type != "cloverleaf" && geometry.geometry_type != "grid" {
            return Err(anyhow!("Only 'donut', 'cloverleaf', and 'grid' geometry types are currently supported"));
        }
        
        // Validate grid-specific fields
        if geometry.geometry_type == "grid" {
            if let Some(grid) = &geometry.grid {
                if grid.is_empty() {
                    return Err(anyhow!("Grid cannot be empty"));
                }
                
                let row_length = grid[0].len();
                for (i, row) in grid.iter().enumerate() {
                    if row.len() != row_length {
                        return Err(anyhow!("All grid rows must have the same length. Row {} has length {}, expected {}", i, row.len(), row_length));
                    }
                }
                
                if let Some(cell_size) = geometry.cell_size {
                    if cell_size <= 0.0 {
                        return Err(anyhow!("Cell size must be positive"));
                    }
                } else {
                    return Err(anyhow!("Cell size is required for grid geometry"));
                }
                
                // Validate spawn points are within grid bounds
                if let Some(spawn_points) = &geometry.spawn_points {
                    for point in spawn_points {
                        if point.row >= grid.len() || point.col >= row_length {
                            return Err(anyhow!("Spawn point {} is out of grid bounds at ({}, {})", point.id, point.row, point.col));
                        }
                    }
                }
                
                // Validate exit points are within grid bounds
                if let Some(exit_points) = &geometry.exit_points {
                    for point in exit_points {
                        if point.row >= grid.len() || point.col >= row_length {
                            return Err(anyhow!("Exit point {} is out of grid bounds at ({}, {})", point.id, point.row, point.col));
                        }
                    }
                }
            } else {
                return Err(anyhow!("Grid is required for grid geometry type"));
            }
        } else {
            // Non-grid geometry validation
            if geometry.inner_radius >= geometry.outer_radius {
                return Err(anyhow!("Inner radius must be less than outer radius"));
            }
            
            if geometry.lane_width <= 0.0 || geometry.lane_count == 0 {
                return Err(anyhow!("Lane width and count must be positive"));
            }
        }
        
        // For grid geometry, ensure basic lane properties are still valid
        if geometry.geometry_type == "grid" && geometry.lane_width <= 0.0 {
            return Err(anyhow!("Lane width must be positive even for grid geometry"));
        }
        
        // Validate entry points
        for entry in &self.route.entries {
            if entry.lane == 0 || entry.lane > geometry.lane_count {
                return Err(anyhow!("Entry lane {} is out of range (1-{})", entry.lane, geometry.lane_count));
            }
            
            if entry.angle < 0.0 || entry.angle >= 360.0 {
                return Err(anyhow!("Entry angle {} must be in range [0, 360)", entry.angle));
            }
        }
        
        // Validate exit points
        for exit in &self.route.exits {
            if exit.lane == 0 || exit.lane > geometry.lane_count {
                return Err(anyhow!("Exit lane {} is out of range (1-{})", exit.lane, geometry.lane_count));
            }
            
            if exit.angle < 0.0 || exit.angle >= 360.0 {
                return Err(anyhow!("Exit angle {} must be in range [0, 360)", exit.angle));
            }
        }
        
        // Validate traffic rules
        let rules = &self.route.traffic_rules;
        if rules.speed_limit <= 0.0 || rules.min_speed <= 0.0 {
            return Err(anyhow!("Speed limits must be positive"));
        }
        
        if rules.min_speed >= rules.speed_limit {
            return Err(anyhow!("Minimum speed must be less than speed limit"));
        }
        
        if rules.following_distance <= 0.0 || rules.lane_change_time <= 0.0 {
            return Err(anyhow!("Following distance and lane change time must be positive"));
        }
        
        // Validate surface properties
        let surface = &self.route.surface;
        if surface.friction_coefficient <= 0.0 || surface.friction_coefficient > 1.0 {
            return Err(anyhow!("Friction coefficient must be in range (0, 1]"));
        }
        
        Ok(())
    }
}