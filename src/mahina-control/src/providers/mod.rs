pub mod package;
pub mod service;
pub mod storage;
pub mod system;
pub mod user;

use crate::error::ControlError;
use crate::protocol::{DomainCommand, DomainResult};
use self::package::PackageProvider;
use self::service::ServiceProvider;
use self::storage::StorageProvider;
use self::system::SystemProvider;
use self::user::UserProvider;

pub struct ProviderDispatcher {
    pub system: SystemProvider,
    pub service: ServiceProvider,
    pub user: UserProvider,
    pub package: PackageProvider,
    pub storage: StorageProvider,
}

impl ProviderDispatcher {
    pub fn new() -> Self {
        Self {
            system: SystemProvider::new(),
            service: ServiceProvider::new(),
            user: UserProvider::new(),
            package: PackageProvider::new(),
            storage: StorageProvider::new(),
        }
    }

    pub fn with_providers(
        system: SystemProvider,
        service: ServiceProvider,
        user: UserProvider,
        package: PackageProvider,
        storage: StorageProvider,
    ) -> Self {
        Self {
            system,
            service,
            user,
            package,
            storage,
        }
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
            DomainCommand::ServicesList => {
                let services = self.service.list()?;
                Ok(DomainResult::ServicesList(services))
            }
            DomainCommand::ServicesStatus { name } => {
                let status = self.service.status(name.as_deref())?;
                Ok(DomainResult::ServiceStatus(status))
            }
            DomainCommand::ServicesStart { name } => {
                self.service.start(&name)?;
                Ok(DomainResult::SuccessMessage(format!("Service '{}' started successfully", name)))
            }
            DomainCommand::ServicesStop { name } => {
                self.service.stop(&name)?;
                Ok(DomainResult::SuccessMessage(format!("Service '{}' stopped successfully", name)))
            }
            DomainCommand::ServicesRestart { name } => {
                self.service.restart(&name)?;
                Ok(DomainResult::SuccessMessage(format!("Service '{}' restarted successfully", name)))
            }
            DomainCommand::ServicesReload { name } => {
                self.service.reload(&name)?;
                Ok(DomainResult::SuccessMessage(format!("Service '{}' reloaded successfully", name)))
            }
            DomainCommand::UsersList => {
                let users = self.user.list()?;
                Ok(DomainResult::UsersList(users))
            }
            DomainCommand::PackagesList => {
                let packages = self.package.list()?;
                Ok(DomainResult::PackagesList(packages))
            }
            DomainCommand::PackagesInstall { target } => {
                let msg = self.package.install(&target)?;
                Ok(DomainResult::SuccessMessage(msg))
            }
            DomainCommand::PackagesRemove { name } => {
                let msg = self.package.remove(&name)?;
                Ok(DomainResult::SuccessMessage(msg))
            }
            DomainCommand::StorageList => {
                let overview = self.storage.get_overview()?;
                Ok(DomainResult::StorageOverview(overview))
            }
            DomainCommand::StorageMount {
                source,
                target,
                fs_type,
                options,
            } => {
                let msg = self.storage.mount(&source, &target, fs_type.as_deref(), options.as_deref())?;
                Ok(DomainResult::SuccessMessage(msg))
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
