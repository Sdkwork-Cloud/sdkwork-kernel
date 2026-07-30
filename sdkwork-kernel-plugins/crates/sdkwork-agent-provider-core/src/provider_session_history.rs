use std::path::Path;

use sdkwork_agent_kernel::{KernelError, KernelResult};

const DEFAULT_MAX_SOURCE_RECORDS: usize = 50_000;
const DEFAULT_MAX_MESSAGES: usize = 10_000;
const DEFAULT_MAX_SERIALIZED_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderSessionHistoryLimits {
    pub max_source_records: usize,
    pub max_messages: usize,
    pub max_serialized_bytes: usize,
}

impl Default for ProviderSessionHistoryLimits {
    fn default() -> Self {
        Self {
            max_source_records: DEFAULT_MAX_SOURCE_RECORDS,
            max_messages: DEFAULT_MAX_MESSAGES,
            max_serialized_bytes: DEFAULT_MAX_SERIALIZED_BYTES,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProviderSessionHistoryBudget {
    limits: ProviderSessionHistoryLimits,
    source_records: usize,
    messages: usize,
    serialized_bytes: usize,
}

impl Default for ProviderSessionHistoryBudget {
    fn default() -> Self {
        Self::new(ProviderSessionHistoryLimits::default())
    }
}

impl ProviderSessionHistoryBudget {
    pub fn new(limits: ProviderSessionHistoryLimits) -> Self {
        Self {
            limits,
            source_records: 0,
            messages: 0,
            serialized_bytes: 0,
        }
    }

    pub fn limits(&self) -> ProviderSessionHistoryLimits {
        self.limits
    }

    pub fn validate_file_size(&self, path: &Path) -> KernelResult<()> {
        let size = std::fs::metadata(path)
            .map_err(|error| {
                KernelError::provider_error(
                    "provider_history_metadata",
                    format!("provider history metadata failed: {error}"),
                )
            })?
            .len();
        if size > self.limits.max_serialized_bytes as u64 {
            return Err(KernelError::validation(format!(
                "provider session history exceeds {} serialized bytes",
                self.limits.max_serialized_bytes
            )));
        }
        Ok(())
    }

    pub fn record_source(&mut self, serialized_bytes: usize) -> KernelResult<()> {
        self.source_records = self.source_records.checked_add(1).ok_or_else(|| {
            KernelError::validation("provider session history source record count overflow")
        })?;
        if self.source_records > self.limits.max_source_records {
            return Err(KernelError::validation(format!(
                "provider session history exceeds {} source records",
                self.limits.max_source_records
            )));
        }
        self.serialized_bytes = self
            .serialized_bytes
            .checked_add(serialized_bytes)
            .ok_or_else(|| {
                KernelError::validation("provider session history byte count overflow")
            })?;
        if self.serialized_bytes > self.limits.max_serialized_bytes {
            return Err(KernelError::validation(format!(
                "provider session history exceeds {} serialized bytes",
                self.limits.max_serialized_bytes
            )));
        }
        Ok(())
    }

    pub fn record_message(&mut self) -> KernelResult<()> {
        self.messages = self.messages.checked_add(1).ok_or_else(|| {
            KernelError::validation("provider session history message count overflow")
        })?;
        if self.messages > self.limits.max_messages {
            return Err(KernelError::validation(format!(
                "provider session history exceeds {} messages",
                self.limits.max_messages
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_rejects_source_message_and_byte_overflow() {
        let limits = ProviderSessionHistoryLimits {
            max_source_records: 1,
            max_messages: 1,
            max_serialized_bytes: 4,
        };

        let mut source_budget = ProviderSessionHistoryBudget::new(limits);
        source_budget.record_source(4).expect("first source");
        assert!(source_budget.record_source(0).is_err());

        let mut message_budget = ProviderSessionHistoryBudget::new(limits);
        message_budget.record_message().expect("first message");
        assert!(message_budget.record_message().is_err());

        let mut byte_budget = ProviderSessionHistoryBudget::new(limits);
        assert!(byte_budget.record_source(5).is_err());
    }
}
