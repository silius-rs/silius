#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub enum Entity {
    Account,
    Factory,
    Paymaster,
    Aggregator,
    EntryPoint,
    #[default]
    Unknown,
}

impl core::fmt::Display for Entity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Entity::Account => "account",
            Entity::Factory => "factory",
            Entity::Paymaster => "paymaster",
            Entity::Aggregator => "aggregator",
            Entity::EntryPoint => "entrypoint",
            Entity::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}
