use sdkwork_agent_kernel::{KernelError, KernelErrorSource, KernelResult};
use sdkwork_id_core::{
    default_snowflake_epoch_millis, max_snowflake_node_id, SnowflakeIdError, SnowflakeIdGenerator,
};

const DEFAULT_AGENT_BUSINESS_NODE_ID: u16 = 1;
const SEQUENCE_BITS: u8 = 12;
const NODE_BITS: u8 = 10;
const NODE_MASK: u64 = (1_u64 << NODE_BITS) - 1;
const TIMESTAMP_SHIFT: u8 = NODE_BITS + SEQUENCE_BITS;

pub trait AgentIdGenerator {
    fn next_id(&self) -> KernelResult<u64>;
}

#[derive(Debug, Clone)]
pub struct AgentBusinessIdGenerator {
    snowflake: SnowflakeIdGenerator,
}

impl AgentBusinessIdGenerator {
    pub fn new() -> KernelResult<Self> {
        Self::with_node_id(DEFAULT_AGENT_BUSINESS_NODE_ID)
    }

    pub fn with_node_id(node_id: u16) -> KernelResult<Self> {
        let snowflake = SnowflakeIdGenerator::new(node_id).map_err(map_snowflake_error)?;
        Ok(Self { snowflake })
    }

    pub fn with_snowflake(snowflake: SnowflakeIdGenerator) -> Self {
        Self { snowflake }
    }

    pub fn node_id(&self) -> u16 {
        self.snowflake.node_id()
    }

    pub fn epoch_millis(&self) -> u64 {
        self.snowflake.epoch_millis()
    }

    pub fn default_epoch_millis() -> u64 {
        default_snowflake_epoch_millis()
    }

    pub fn max_node_id() -> u16 {
        max_snowflake_node_id()
    }

    pub fn decode_node_id(id: u64) -> u16 {
        ((id >> SEQUENCE_BITS) & NODE_MASK) as u16
    }

    pub fn decode_timestamp_delta_millis(id: u64) -> u64 {
        id >> TIMESTAMP_SHIFT
    }

    pub fn new_default() -> KernelResult<Self> {
        Self::with_node_id(DEFAULT_AGENT_BUSINESS_NODE_ID)
    }
}

impl AgentIdGenerator for AgentBusinessIdGenerator {
    fn next_id(&self) -> KernelResult<u64> {
        let value = self.snowflake.generate().map_err(map_snowflake_error)?;
        if value <= 0 {
            return Err(KernelError::validation(
                "snowflake id generator returned a non-positive id",
            ));
        }
        Ok(value as u64)
    }
}

fn map_snowflake_error(error: SnowflakeIdError) -> KernelError {
    let message = format!("failed to generate snowflake id: {error:?}");
    match error {
        SnowflakeIdError::ClockMovedBackwards { .. } => {
            KernelError::conflict(message).from_source(KernelErrorSource::Runtime)
        }
        SnowflakeIdError::SequenceExhausted { .. } => {
            KernelError::resource_exhausted(message).from_source(KernelErrorSource::Runtime)
        }
        _ => KernelError::validation(message).from_source(KernelErrorSource::Runtime),
    }
}
