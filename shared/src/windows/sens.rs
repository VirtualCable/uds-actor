use windows::{
    Win32::{
        Foundation::E_NOTIMPL,
        System::{
            Com::{
                CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, DISPATCH_FLAGS,
                DISPPARAMS, EXCEPINFO,
                Events::{IEventSubscription, IEventSystem},
                IDispatch, IDispatch_Impl, ITypeInfo, StringFromCLSID,
            },
            EventNotificationService::{ISensLogon, ISensLogon_Impl},
            Variant::VARIANT,
        },
    },
    core::*,
};

use windows::core::GUID;

use crate::log;

// SENS publisher GUID
pub const SENSGUID_PUBLISHER: GUID = GUID::from_u128(0x5fee1bd6_5b9b_11d1_8dd2_00aa004abd5e);

// SENS Logon event class GUID
pub const SENSGUID_EVENTCLASS_LOGON: GUID = GUID::from_u128(0xd5978630_5b9f_11d1_8dd2_00aa004abd5e);

#[allow(non_upper_case_globals)]
pub const CLSID_CEventSystem: GUID = GUID::from_u128(0x4e14fba2_2e22_11d1_9964_00c04fbbb345);

#[allow(non_upper_case_globals)]
pub const CLSID_CEventSubscription: GUID = GUID::from_u128(0x7542e960_79c7_11d1_88f9_0080c7d771bf);

#[implement(ISensLogon, IDispatch)]
#[derive(Clone, Default)]
pub struct SensLogon {}

impl Drop for SensLogon {
    fn drop(&mut self) {}
}

impl IDispatch_Impl for SensLogon_Impl {
    fn GetTypeInfoCount(&self) -> windows_core::Result<u32> {
        Ok(0)
    }
    fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> windows_core::Result<ITypeInfo> {
        Err(E_NOTIMPL.into())
    }
    fn GetIDsOfNames(
        &self,
        _riid: *const windows_core::GUID,
        _rgsznames: *const windows_core::PCWSTR,
        _cnames: u32,
        _lcid: u32,
        _rgdispid: *mut i32,
    ) -> windows_core::Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn Invoke(
        &self,
        _dispidmember: i32,
        _riid: *const windows_core::GUID,
        _lcid: u32,
        _wflags: DISPATCH_FLAGS,
        _pdispparams: *const DISPPARAMS,
        _pvarresult: *mut VARIANT,
        _pexcepinfo: *mut EXCEPINFO,
        _puargerr: *mut u32,
    ) -> windows_core::Result<()> {
        Err(E_NOTIMPL.into())
    }
}

impl ISensLogon_Impl for SensLogon_Impl {
    fn Logon(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn Logoff(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn StartShell(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn DisplayLock(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn DisplayUnlock(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn StartScreenSaver(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn StopScreenSaver(&self, _bstrusername: &windows_core::BSTR) -> windows_core::Result<()> {
        Ok(())
    }
}

// --- Register subscription in EventSystem ---
pub fn register_sens_subscription() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let event_system: IEventSystem = CoCreateInstance(&CLSID_CEventSystem, None, CLSCTX_ALL)?;
        let subscription: IEventSubscription =
            CoCreateInstance(&CLSID_CEventSubscription, None, CLSCTX_ALL)?;

        subscription.SetSubscriptionName(&BSTR::from("UDS SENS Logon Subscription"))?;
        subscription.SetEventClassID(&BSTR::from("SensLogon"))?;

        let pwstr = StringFromCLSID(&SENSGUID_PUBLISHER)?;
        // `pwstr` es un PWSTR, conviértelo a String
        let guid_string: String = pwstr.to_string()?;
        // Ahora sí, conviértelo a BSTR
        let bstr = BSTR::from(guid_string);
        subscription.SetPublisherID(&bstr)?;
        subscription.SetMethodName(&BSTR::from(""))?;

        let sink: IDispatch = SensLogon::default().into();
        subscription.SetSubscriberInterface(&sink)?;
        subscription.SetEnabled(true)?;

        // Disambiguate conversion: IEventSystem::Store needs IUnknown
        let unk: IUnknown = subscription.clone().into();
        event_system.Store(&BSTR::from("Subscription"), &unk)?;

        log::info!("SENS subscription registered");

        // TODO: fix this
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}
