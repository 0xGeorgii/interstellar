#![no_std]
use soroban_sdk::{vec, Vec, Env};

/// Represents different stages for timelock settings.
#[derive(Clone, Copy, PartialEq)]
pub enum Stage {
    SrcWithdrawal,
    SrcPublicWithdrawal,
    SrcCancellation,
    SrcPublicCancellation,
    DstWithdrawal,
    DstPublicWithdrawal,
    DstCancellation,
}

impl From<Stage> for u32 {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::SrcWithdrawal => 1,
            Stage::SrcPublicWithdrawal => 2,
            Stage::SrcCancellation => 3,
            Stage::SrcPublicCancellation => 4,
            Stage::DstWithdrawal => 5,
            Stage::DstPublicWithdrawal => 6,
            Stage::DstCancellation => 7,
        }
    }
}

/// Public type representing Timelocks, backed by a Vec<u32>
pub struct Timelocks(Vec<u32>);

impl Timelocks {
    pub fn new(env: &Env, deployed_at: u32) -> Self {
        let mut data = vec![env, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]; // 8 elements: [deployed_at, 7 stages]
        data.set(0, deployed_at);
        Self(data)
    }

    /// Set deployment timestamp
    pub fn set_deployed_at(&mut self, value: u32) {
        self.0.set(0, value);
    }

    /// Get deployment timestamp
    pub fn deployed_at(&self) -> u32 {
        self.0.get(0).unwrap_or(0)
    }

    /// Sets the delay for a specific stage (value is seconds from deploy time)
    pub fn set_stage(&mut self, stage: Stage, value: u32) {
        let idx: u32 = stage.into();
        self.0.set(idx, value);
    }

    /// Gets the absolute time when the given stage starts
    pub fn get(&self, stage: Stage) -> u32 {
        let idx: u32 = stage.into();
        // Absolute time = deployed_at + delay (in seconds)
        self.deployed_at() + self.0.get(idx).unwrap_or(0)
    }

    /// Computes the start of the rescue period: deploy_time + delay
    pub fn rescue_start(&self, rescue_delay: u32) -> u32 {
        self.deployed_at() + rescue_delay
    }
}

mod test;
