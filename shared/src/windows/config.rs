use anyhow::Result;
use base64::engine::{Engine as _, general_purpose::STANDARD};
use windows::{
    Win32::{
        Foundation::HANDLE,
        Security::{
            ACL,
            Authorization::{GetSecurityInfo, SE_REGISTRY_KEY, SetSecurityInfo},
            CreateWellKnownSid, DACL_SECURITY_INFORMATION, DeleteAce, EqualSid,
            PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SID,
            WinBuiltinUsersSid,
        },
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_QUERY_VALUE,
            REG_BINARY, REG_CREATE_KEY_DISPOSITION, REG_OPTION_NON_VOLATILE, RegCloseKey,
            RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    config::{ActorConfiguration, ConfigLoader},
    log,
};

// Constants
const PATH: PCWSTR = w!("SOFTWARE\\UDSActor");

unsafe fn fix_registry_permissions(hkey: HKEY) -> Result<()> {
    let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();
    let mut p_dacl: *mut ACL = std::ptr::null_mut();

    // Get current security info
    unsafe {
        let result = GetSecurityInfo(
            HANDLE(hkey.0),
            SE_REGISTRY_KEY,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_dacl),
            None,
            Some(&mut p_sd),
        );
        if result.is_err() {
            return Err(anyhow::anyhow!("GetSecurityInfo failed: {}", result.0));
        }
    }

    // Remove User (1-5-32-545, BUILTIN\Users) sid from DACL to prevent access by standard users
    let ace_count = unsafe { (*p_dacl).AceCount };
    let mut sid_buf = [0u8; 68]; // enough space for a SID
    let mut sid_size = sid_buf.len() as u32;
    let sid_ptr_builtin = sid_buf.as_mut_ptr() as *mut SID;
    unsafe {
        if CreateWellKnownSid(
            WinBuiltinUsersSid,
            None,
            Some(PSID(sid_ptr_builtin as *mut _)),
            &mut sid_size,
        )
        .is_err()
        {
            return Err(anyhow::anyhow!("CreateWellKnownSid failed"));
        }
    }

    for i in 0..ace_count {
        let mut p_ace = std::ptr::null_mut();
        if unsafe { windows::Win32::Security::GetAce(p_dacl, i as u32, &mut p_ace).is_err() } {
            continue;
        }
        if p_ace.is_null() {
            continue;
        }
        let ace = unsafe { *(p_ace as *const windows::Win32::Security::ACCESS_ALLOWED_ACE) };
        let sid_ptr = &ace.SidStart as *const u32 as *const SID;
        if unsafe { EqualSid(PSID(sid_ptr as *mut _), PSID(sid_ptr_builtin as *mut _)).is_ok() } {
            // Found BUILTIN\Users ACE, remove it
            if unsafe { DeleteAce(p_dacl, i as u32).is_err() } {
                return Err(anyhow::anyhow!("DeleteAce failed"));
            }
            break;
        }
    }

    unsafe {
        let result = SetSecurityInfo(
            HANDLE(hkey.0),
            SE_REGISTRY_KEY,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(p_dacl),
            None,
        );
        if result.is_err() {
            return Err(anyhow::anyhow!("SetSecurityInfo failed: {}", result.0));
        }
    }

    Ok(())
}

// Allow to choose registry root, for testing purposes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryRoot {
    LocalMachine,
    CurrentUser,
}

impl RegistryRoot {
    pub fn as_hkey(&self) -> HKEY {
        match self {
            RegistryRoot::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryRoot::CurrentUser => HKEY_CURRENT_USER,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowsConfig {
    actor: Option<ActorConfiguration>,
    keys_root: RegistryRoot,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            actor: None,
            keys_root: RegistryRoot::LocalMachine,
        }
    }
}

impl ConfigLoader for WindowsConfig {
    fn load_config(&mut self) -> Result<ActorConfiguration> {
        // Try to open the registry key for reading
        unsafe {
            let mut hkey: HKEY = HKEY::default();
            let status = RegOpenKeyExW(
                self.keys_root.as_hkey(),
                PATH,
                None,
                KEY_QUERY_VALUE,
                &mut hkey,
            );

            if status.is_err() {
                // If key does not exist, return a default configuration
                return Ok(ActorConfiguration::default());
            }

            // Query the unnamed (default) value
            let mut buf = [0u8; 4096];
            let mut buf_len: u32 = buf.len() as u32;
            let status = RegQueryValueExW(
                hkey,
                None,
                None,
                None,
                Some(buf.as_mut_ptr()),
                Some(&mut buf_len),
            );

            _ = RegCloseKey(hkey);

            if status.is_err() {
                return Ok(ActorConfiguration::default());
            }

            // Decode base64 and parse JSON
            let data = &buf[..buf_len as usize];
            let decoded = STANDARD
                .decode(data)
                .map_err(|e| anyhow::anyhow!("Base64 decode failed: {e}"))?;
            let cfg: ActorConfiguration = serde_json::from_slice(&decoded)
                .map_err(|e| anyhow::anyhow!("JSON parse failed: {e}"))?;

            // Store for future use
            self.actor = Some(cfg.clone());

            Ok(cfg)
        }
    }

    // Note: Does not creates the intermediate keys, they must exist
    // So the installer must create them or use a PATH that is sure to exist (e.g. SOFTWARE)
    // The final key (UDSActor) will be created if not existing
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()> {
        log::debug!("Saving configuration to registry: {:?}", config);
        // Serialize config to JSON and encode as base64
        let json = serde_json::to_vec(config)?;
        let encoded = STANDARD.encode(json);

        unsafe {
            // Try to open or create the registry key
            let mut hkey: HKEY = HKEY::default();
            let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
            let status = RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                PATH,
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                None,
                &mut hkey,
                Some(&mut disposition),
            );

            // IF already exists, open with all access
            if status.is_err() {
                let status = RegOpenKeyExW(
                    self.keys_root.as_hkey(),
                    PATH,
                    None,
                    KEY_ALL_ACCESS,
                    &mut hkey,
                );
                if status.is_err() {
                    return Err(anyhow::anyhow!("Failed to open or create registry key"));
                }
            }

            // Apply security fix (remove BUILTIN\Users ACE)
            // if using LocalMachine root
            if self.keys_root == RegistryRoot::LocalMachine {
                fix_registry_permissions(hkey)?;
            }

            // Write the unnamed (default) value as REG_BINARY
            let data = encoded.as_bytes();
            let status = RegSetValueExW(hkey, None, None, REG_BINARY, Some(data));

            _ = RegCloseKey(hkey);

            if status.is_err() {
                return Err(anyhow::anyhow!("Failed to write registry value"));
            }
        }
        self.actor = Some(config.clone());
        Ok(())
    }

    fn clear_config(&mut self) -> Result<()> {
        unsafe {
            // Try to open the registry key
            let mut hkey: HKEY = HKEY::default();
            let status = RegOpenKeyExW(
                self.keys_root.as_hkey(),
                PATH,
                None,
                KEY_ALL_ACCESS,
                &mut hkey,
            );

            if status.is_err() {
                return Err(anyhow::anyhow!("Failed to open registry key"));
            }

            // Delete the unnamed (default) value
            let status = RegDeleteValueW(hkey, None);

            _ = RegCloseKey(hkey);

            if status.is_err() {
                return Err(anyhow::anyhow!("Failed to delete registry value"));
            }
        }
        self.actor = None;
        Ok(())
    }

    fn config(&mut self, force_reload: bool) -> Result<ActorConfiguration> {
        if force_reload || self.actor.is_none() {
            self.load_config()
        } else {
            Ok(self.actor.clone().unwrap())
        }
    }
}

pub fn new_config_loader() -> Box<dyn ConfigLoader> {
    Box::new(WindowsConfig::default())
}

#[cfg(test)]
pub fn new_test_config_loader() -> Box<dyn ConfigLoader> {
    Box::new(WindowsConfig {
        actor: None,
        keys_root: RegistryRoot::CurrentUser,
    })
}
