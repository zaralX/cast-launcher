mod installer;
mod processors;

pub use installer::{Installer, Processor};
pub use processors::ProcessorEnv;

use crate::error::CommandResult;

pub async fn build_client<S, C>(
    installer: &Installer,
    env: &ProcessorEnv<'_>,
    on_step: S,
    cancelled: C,
) -> CommandResult<()>
where
    S: Fn(usize, usize, &str),
    C: Fn() -> bool,
{
    processors::run(
        installer.processors(),
        installer.data(),
        env,
        on_step,
        cancelled,
    )
    .await
}
