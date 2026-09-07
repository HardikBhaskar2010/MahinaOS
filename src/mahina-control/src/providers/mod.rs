pub mod system;

use crate::error::ControlError;
use crate::protocol::{DomainCommand, DomainResult};
use self::system::SystemProvider;

pub struct ProviderDispatcher {
    pub system: SystemProvider,
}

impl ProviderDispatcher {
    pub fn new() -> Self {
        Self {
            system: SystemProvider::new(),
        }
    }

    pub fn with_system(system: SystemProvider) -> Self {
        Self { system }
    }

    pub fn dispatch(&self, command: DomainCommand) -> Result<DomainResult, ControlError> {
        match command {
            DomainCommand::SystemGetState => {
                let state = self.system.get_state()?;
                Ok(DomainResult::SystemState(state))
            }
            DomainCommand::SystemReboot => {
                self.system.reboot()?;
                Ok(DomainResult::SuccessMessage("Reboot initiated".to_string()))
            }
            DomainCommand::SystemShutdown => {
                self.system.shutdown()?;
                Ok(DomainResult::SuccessMessage("Shutdown initiated".to_string()))
            }
        }
    }
}

impl Default for ProviderDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_system_get_state() {
        let dispatcher = ProviderDispatcher::new();
        let res = dispatcher.dispatch(DomainCommand::SystemGetState).expect("dispatch failed");
        match res {
            DomainResult::SystemState(s) => {
                assert!(s.active_generation >= 100);
            }
            _ => panic!("unexpected domain result"),
        }
    }
}
