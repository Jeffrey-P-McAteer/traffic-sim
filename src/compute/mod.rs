use crate::simulation::SimulationState;
use anyhow::Result;

pub mod gpu;
pub mod cpu;

pub use cpu::*;
pub use gpu::*;

pub trait SimulationBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()>;
    fn get_name(&self) -> &'static str;
    fn supports_gpu(&self) -> bool;
}

pub enum ComputeBackend {
    Cpu(CpuBackend),
    Gpu(GpuBackend),
}

impl ComputeBackend {
    pub fn new_cpu(
        cars_config: crate::config::CarsConfig, 
        route_config: crate::config::RouteConfig,
        seed: Option<u64>
    ) -> Self {
        ComputeBackend::Cpu(CpuBackend::new(cars_config, route_config, seed))
    }
    
    pub fn new_gpu(
        cars_config: crate::config::CarsConfig, 
        route_config: crate::config::RouteConfig,
        seed: Option<u64>
    ) -> Result<Self> {
        Ok(ComputeBackend::Gpu(GpuBackend::new(cars_config, route_config, seed)?))
    }
}

impl SimulationBackend for ComputeBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()> {
        match self {
            ComputeBackend::Cpu(backend) => backend.update(state),
            ComputeBackend::Gpu(backend) => backend.update(state),
        }
    }
    
    fn get_name(&self) -> &'static str {
        match self {
            ComputeBackend::Cpu(backend) => backend.get_name(),
            ComputeBackend::Gpu(backend) => backend.get_name(),
        }
    }
    
    fn supports_gpu(&self) -> bool {
        match self {
            ComputeBackend::Cpu(backend) => backend.supports_gpu(),
            ComputeBackend::Gpu(backend) => backend.supports_gpu(),
        }
    }
}

impl ComputeBackend {
    pub fn spawn_manual_car(&mut self, behavior_name: &str, state: &mut SimulationState) {
        match self {
            ComputeBackend::Cpu(backend) => backend.spawn_manual_car(behavior_name, state),
            ComputeBackend::Gpu(backend) => backend.spawn_manual_car(behavior_name, state),
        }
    }
    
    pub fn mark_car_for_exit(&mut self, behavior_name: &str, state: &mut SimulationState) -> bool {
        // This is handled directly in the simulation state
        state.mark_car_for_exit(behavior_name)
    }
}