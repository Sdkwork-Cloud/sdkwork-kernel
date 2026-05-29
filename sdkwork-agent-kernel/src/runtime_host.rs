use crate::{
    AgentRuntime, AgentRuntimeConformanceProfile, AgentRuntimeDiagnostics, KernelConformanceReport,
    KernelError, KernelResult,
};

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
}
