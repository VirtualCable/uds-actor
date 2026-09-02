use anyhow::Result;

use windows::{Win32::Foundation::*, Win32::System::Services::*, core::*};

use crate::log;

pub fn register(name: &str, display_name: &str, description: &str) -> Result<()> {
    log::info!("Registering service: {}", name);

    let name = widestring::U16CString::from_str_truncate(name);
    let display_name = widestring::U16CString::from_str_truncate(display_name);
    let description = widestring::U16CString::from_str_truncate(description);
    let bin_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to get current exe path: {}", e))?
        .to_string_lossy()
        .to_string();

    let bin_path = widestring::U16CString::from_str_truncate(bin_path);

    unsafe {
        if let Ok(scm) = OpenSCManagerW(None, None, SC_MANAGER_ALL_ACCESS) {
            if let Ok(service) = CreateServiceW(
                scm,
                PCWSTR(name.as_ptr()),
                PCWSTR(display_name.as_ptr()),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(bin_path.as_ptr()),
                None,
                None,
                None,
                None,
                None,
            ) {
                // Set service description
                let mut sd = SERVICE_DESCRIPTIONW {
                    lpDescription: PWSTR(description.as_ptr() as *mut _),
                };
                ChangeServiceConfig2W(
                    service,
                    SERVICE_CONFIG_DESCRIPTION,
                    Some(&mut sd as *mut _ as *mut _),
                )?;

                // Config recovery: restart always, delay 5000ms, reset period 86400s
                // 1 action that will be repeated on failure for 1º, 2º, ... failures
                let actions = [SC_ACTION {
                    Type: SC_ACTION_RESTART,
                    Delay: 5000,
                }];

                let mut sfa = SERVICE_FAILURE_ACTIONSW {
                    dwResetPeriod: 86400, // segundos
                    lpRebootMsg: PWSTR::null(),
                    lpCommand: PWSTR::null(),
                    cActions: actions.len() as u32,
                    lpsaActions: actions.as_ptr() as *mut _,
                };

                ChangeServiceConfig2W(
                    service,
                    SERVICE_CONFIG_FAILURE_ACTIONS,
                    Some(&mut sfa as *mut _ as *mut _),
                )?;

                CloseServiceHandle(service)?;
                CloseServiceHandle(scm)?;
                Ok(())
            } else {
                CloseServiceHandle(scm)?;
                Err(anyhow::anyhow!(
                    "Failed to create service: {}",
                    windows::core::Error::from_thread()
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Failed to open service manager: {}",
                windows::core::Error::from_thread()
            ))
        }
    }
}

const STOP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn unregister(name: &str) -> Result<()> {
    log::info!("Unregistering service: {}", name);

    let name_w = widestring::U16CString::from_str_truncate(name);

    unsafe {
        let scm = OpenSCManagerW(None, None, SC_MANAGER_ALL_ACCESS)?;
        let service = match OpenServiceW(scm, PCWSTR(name_w.as_ptr()), SERVICE_ALL_ACCESS) {
            Ok(service) => service,
            Err(err) => {
                let _ = CloseServiceHandle(scm);
                return if err.code() == ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() {
                    log::info!("Service {} is not registered, nothing to remove", name);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to open service {}: {}", name, err))
                };
            }
        };

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        if let Err(err) = ControlService(service, SERVICE_CONTROL_STOP, &mut status) {
            // An already stopped service reports ERROR_SERVICE_NOT_ACTIVE, and a service
            // that refuses to stop still has to be deleted, so neither aborts the removal
            if err.code() != ERROR_SERVICE_NOT_ACTIVE.to_hresult() {
                log::warn!("Could not stop service {}: {}", name, err);
            }
        }

        wait_until_stopped(service, STOP_TIMEOUT);

        let deleted = DeleteService(service)
            .map_err(|err| anyhow::anyhow!("Failed to delete service {}: {}", name, err));

        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(scm);

        deleted
    }
}

unsafe fn wait_until_stopped(service: SC_HANDLE, timeout: std::time::Duration) {
    let deadline = std::time::Instant::now() + timeout;

    while std::time::Instant::now() < deadline {
        let mut buf = [0u8; std::mem::size_of::<SERVICE_STATUS_PROCESS>()];
        let mut needed = 0u32;

        let queried = unsafe {
            QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, Some(&mut buf), &mut needed)
        };
        if queried.is_err() {
            return;
        }

        let status: SERVICE_STATUS_PROCESS = unsafe { std::ptr::read(buf.as_ptr() as *const _) };
        if status.dwCurrentState == SERVICE_STOPPED {
            return;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log::warn!("Timed out waiting for the service to stop, deleting it anyway");
}
