use crate::simulation::{SimulationState, Car, CarId};
use anyhow::Result;

#[cfg(feature = "gpu-sim")]
pub mod gpu;

pub mod cpu;

pub use cpu::*;

#[cfg(feature = "gpu-sim")]
pub use gpu::*;

pub trait SimulationBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()>;
    fn get_name(&self) -> &'static str;
    fn supports_gpu(&self) -> bool;
}

pub enum ComputeBackend {
    Cpu(CpuBackend),
    #[cfg(feature = "gpu-sim")]
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
    
    #[cfg(feature = "gpu-sim")]
    pub fn new_gpu(
        cars_config: crate::config::CarsConfig, 
        route_config: crate::config::RouteConfig,
        seed: Option<u64>
    ) -> Result<Self> {
        Ok(ComputeBackend::Gpu(GpuBackend::new(cars_config, route_config, seed)?))
    }
    
    #[cfg(not(feature = "gpu-sim"))]
    pub fn new_gpu(
        _cars_config: crate::config::CarsConfig, 
        _route_config: crate::config::RouteConfig,
        _seed: Option<u64>
    ) -> Result<Self> {
        anyhow::bail!("GPU compute not compiled in. Enable 'gpu-sim' feature.")
    }
}

impl SimulationBackend for ComputeBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()> {
        match self {
            ComputeBackend::Cpu(backend) => backend.update(state),
            #[cfg(feature = "gpu-sim")]
            ComputeBackend::Gpu(backend) => backend.update(state),
        }
    }
    
    fn get_name(&self) -> &'static str {
        match self {
            ComputeBackend::Cpu(backend) => backend.get_name(),
            #[cfg(feature = "gpu-sim")]
            ComputeBackend::Gpu(backend) => backend.get_name(),
        }
    }
    
    fn supports_gpu(&self) -> bool {
        match self {
            ComputeBackend::Cpu(backend) => backend.supports_gpu(),
            #[cfg(feature = "gpu-sim")]
            ComputeBackend::Gpu(backend) => backend.supports_gpu(),
        }
    }
}