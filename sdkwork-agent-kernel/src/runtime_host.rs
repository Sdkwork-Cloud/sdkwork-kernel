use crate::{
    AgentRuntime, AgentRuntimeConformanceProfile, AgentRuntimeDiagnostics, KernelConformanceReport,
    KernelError, KernelResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRuntimeSlotState {
    Loaded,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeExecutionHandle {
    pub runtime_id: String,
    pub agent_id: String,
    pub implementation_id: String,
    pub state: AgentRuntimeSlotState,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeRegistration {
    pub implementation_id: String,
    pub runtime: AgentRuntime,
}

impl AgentRuntimeRegistration {
    pub fn new(implementation_id: impl Into<String>, runtime: AgentRuntime) -> Self {
        Self {
            implementation_id: implementation_id.into(),
            runtime,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeSlot {
    pub runtime_id: String,
    pub agent_id: String,
    pub implementation_id: String,
    pub runtime: AgentRuntime,
    pub state: AgentRuntimeSlotState,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKernelHost {
    host_id: String,
    runtimes: Vec<AgentRuntimeSlot>,
}

impl AgentKernelHost {
    pub fn new(host_id: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            runtimes: Vec::new(),
        }
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }

    pub fn load_runtime(&mut self, registration: AgentRuntimeRegistration) -> KernelResult<()> {
        let capability_manifest = registration.runtime.capability_manifest();
        if self
            .runtime(capability_manifest.runtime_id.as_str())
            .is_some()
        {
            return Err(KernelError::validation(format!(
                "runtime id is already loaded: {}",
                capability_manifest.runtime_id
            )));
        }

        self.runtimes.push(AgentRuntimeSlot {
            runtime_id: capability_manifest.runtime_id.clone(),
            agent_id: capability_manifest.agent_id.clone(),
            implementation_id: registration.implementation_id,
            runtime: registration.runtime,
            state: AgentRuntimeSlotState::Loaded,
            failure_reason: None,
        });
        Ok(())
    }

    pub fn unload_runtime(&mut self, runtime_id: &str) -> KernelResult<AgentRuntimeSlot> {
        let index = self
            .runtimes
            .iter()
            .position(|slot| slot.runtime_id == runtime_id)
            .ok_or_else(|| {
                KernelError::validation(format!("runtime id is not loaded: {runtime_id}"))
            })?;

        if self.runtimes[index].state == AgentRuntimeSlotState::Running {
            return Err(KernelError::conflict(format!(
                "runtime is running and must be stopped before unload: {runtime_id}"
            )));
        }

        Ok(self.runtimes.remove(index))
    }

    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }

    pub fn runtime_ids(&self) -> Vec<String> {
        self.runtimes
            .iter()
            .map(|slot| slot.runtime_id.clone())
            .collect()
    }

    pub fn runtime_slot(&self, runtime_id: &str) -> Option<&AgentRuntimeSlot> {
        self.runtimes
            .iter()
            .find(|slot| slot.runtime_id == runtime_id)
    }

    pub fn runtime(&self, runtime_id: &str) -> Option<&AgentRuntime> {
        self.runtime_slot(runtime_id).map(|slot| &slot.runtime)
    }

    pub fn runtime_state(&self, runtime_id: &str) -> Option<AgentRuntimeSlotState> {
        self.runtime_slot(runtime_id).map(|slot| slot.state)
    }

    pub fn running_runtime_ids(&self) -> Vec<String> {
        self.runtimes
            .iter()
            .filter(|slot| slot.state == AgentRuntimeSlotState::Running)
            .map(|slot| slot.runtime_id.clone())
            .collect()
    }

    pub fn start_runtime(&mut self, runtime_id: &str) -> KernelResult<AgentRuntimeExecutionHandle> {
        let slot = self.runtime_slot_mut(runtime_id)?;
        match slot.state {
            AgentRuntimeSlotState::Loaded | AgentRuntimeSlotState::Stopped => {
                slot.state = AgentRuntimeSlotState::Running;
                slot.failure_reason = None;
                Ok(slot.execution_handle())
            }
            AgentRuntimeSlotState::Running => Ok(slot.execution_handle()),
            AgentRuntimeSlotState::Failed => Err(KernelError::conflict(format!(
                "failed runtime must be unloaded or replaced before restart: {runtime_id}"
            ))),
        }
    }

    pub fn stop_runtime(&mut self, runtime_id: &str) -> KernelResult<AgentRuntimeExecutionHandle> {
        let slot = self.runtime_slot_mut(runtime_id)?;
        match slot.state {
            AgentRuntimeSlotState::Loaded
            | AgentRuntimeSlotState::Running
            | AgentRuntimeSlotState::Stopped => {
                slot.state = AgentRuntimeSlotState::Stopped;
                Ok(slot.execution_handle())
            }
            AgentRuntimeSlotState::Failed => Err(KernelError::conflict(format!(
                "failed runtime cannot be stopped and must be unloaded or replaced: {runtime_id}"
            ))),
        }
    }

    pub fn fail_runtime(
        &mut self,
        runtime_id: &str,
        failure_reason: impl Into<String>,
    ) -> KernelResult<AgentRuntimeExecutionHandle> {
        let slot = self.runtime_slot_mut(runtime_id)?;
        slot.state = AgentRuntimeSlotState::Failed;
        slot.failure_reason = Some(failure_reason.into());
        Ok(slot.execution_handle())
    }

    pub fn diagnostics(&self) -> Vec<AgentRuntimeDiagnostics> {
        self.runtimes
            .iter()
            .map(|slot| slot.runtime.diagnostics())
            .collect()
    }

    pub fn conformance_reports(
        &self,
        profile: AgentRuntimeConformanceProfile,
    ) -> Vec<KernelConformanceReport> {
        self.runtimes
            .iter()
            .map(|slot| slot.runtime.conformance_report(profile))
            .collect()
    }

    fn runtime_slot_mut(&mut self, runtime_id: &str) -> KernelResult<&mut AgentRuntimeSlot> {
        self.runtimes
            .iter_mut()
            .find(|slot| slot.runtime_id == runtime_id)
            .ok_or_else(|| {
                KernelError::validation(format!("runtime id is not loaded: {runtime_id}"))
            })
    }
}

impl AgentRuntimeSlot {
    fn execution_handle(&self) -> AgentRuntimeExecutionHandle {
        AgentRuntimeExecutionHandle {
            runtime_id: self.runtime_id.clone(),
            agent_id: self.agent_id.clone(),
            implementation_id: self.implementation_id.clone(),
            state: self.state,
            failure_reason: self.failure_reason.clone(),
        }
    }
}
