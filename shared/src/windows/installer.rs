use anyhow::Result;

use windows::{Win32::System::Services::*, core::*};

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
                Err(anyhow::anyhow!("Failed to create service: {}", windows::core::Error::from_thread()))
            }
        } else {
            Err(anyhow::anyhow!("Failed to open service manager: {}", windows::core::Error::from_thread()))
        }
    }
}

pub fn unregister(name: &str) -> Result<()> {
    let name = widestring::U16CString::from_str_truncate(name);
    unsafe {
        if let Ok(scm) = OpenSCManagerW(None, None, SC_MANAGER_ALL_ACCESS) {
            if let Ok(service) = OpenServiceW(scm, PCWSTR(name.as_ptr()), SERVICE_ALL_ACCESS) {
                DeleteService(service)?;
                CloseServiceHandle(service)?;
            }
            CloseServiceHandle(scm)?;
        } else {
            return Err(anyhow::anyhow!("Failed to open service manager: {}", windows::core::Error::from_thread()));
        }
    }
    Ok(())
}
