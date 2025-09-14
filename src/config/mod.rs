use anyhow::Result;

pub mod route;
pub mod cars;

pub use route::*;
pub use cars::*;

#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub route: RouteConfig,
    pub cars: CarsConfig,
}

impl SimulationConfig {
    pub fn load_from_files(route_path: &str, cars_path: &str) -> Result<Self> {
        let route_content = std::fs::read_to_string(route_path)?;
        let cars_content = std::fs::read_to_string(cars_path)?;
        
        let route: RouteConfig = toml::from_str(&route_content)?;
        let cars: CarsConfig = toml::from_str(&cars_content)?;
        
        // Validate configurations
        route.validate()?;
        cars.validate()?;
        
        Ok(SimulationConfig { route, cars })
    }
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
}