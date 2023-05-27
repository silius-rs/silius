#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Unsafe,
}
