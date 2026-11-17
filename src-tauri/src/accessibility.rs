//! Standard UI Automation for the custom-drawn plant surface. The HWND host
//! supplies geometry/focus; this provider adds its name and open operation.
#![cfg(windows)]
use tauri::AppHandle;
use windows::{
    core::{implement, IUnknown, IUnknownImpl, Interface, Result, BOOL},
    Win32::{
        Foundation::{E_FAIL, E_NOINTERFACE, HWND, LPARAM, WPARAM},
        System::Variant::VARIANT,
        UI::Accessibility::*,
    },
};

#[implement(IRawElementProviderSimple, IInvokeProvider, IWindowProvider)]
struct PlantProvider {
    hwnd: isize,
    app: AppHandle,
}
impl PlantProvider {
    fn name(&self) -> String {
        let (plant, _, status) = crate::presentation(&self.app);
        let plant = match plant {
            catch_core::growth::Plant::Tulip => "郁金香",
            catch_core::growth::Plant::Sunflower => "向日葵",
            catch_core::growth::Plant::Corn => "玉米",
            catch_core::growth::Plant::Strawberry => "草莓",
            catch_core::growth::Plant::Sprout => "绿芽",
        };
        let status = match status {
            Some(catch_core::tracking::Status::Running) => "正在投入",
            Some(catch_core::tracking::Status::Paused) => "准备继续",
            _ => "今日成长",
        };
        format!("Catch · {plant} · {status}")
    }
}
impl IRawElementProviderSimple_Impl for PlantProvider_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }
    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        if pattern == UIA_InvokePatternId {
            self.to_interface::<IInvokeProvider>().cast()
        } else if pattern == UIA_WindowPatternId {
            self.to_interface::<IWindowProvider>().cast()
        } else {
            Err(E_NOINTERFACE.into())
        }
    }
    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(if property == UIA_ControlTypePropertyId {
            VARIANT::from(UIA_WindowControlTypeId.0)
        } else if property == UIA_NamePropertyId {
            VARIANT::from(self.name().as_str())
        } else if property == UIA_HelpTextPropertyId {
            VARIANT::from("单击打开 Catch，拖动移动，Ctrl 加滚轮缩放，右键还原大小")
        } else if property == UIA_IsKeyboardFocusablePropertyId
            || property == UIA_IsEnabledPropertyId
        {
            VARIANT::from(true)
        } else {
            VARIANT::default()
        })
    }
    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as *mut _)) }
    }
}
impl IInvokeProvider_Impl for PlantProvider_Impl {
    fn Invoke(&self) -> Result<()> {
        crate::expand(&self.app);
        Ok(())
    }
}
impl IWindowProvider_Impl for PlantProvider_Impl {
    fn SetVisualState(&self, state: WindowVisualState) -> Result<()> {
        if state == WindowVisualState_Normal {
            Ok(())
        } else {
            Err(E_NOINTERFACE.into())
        }
    }
    fn Close(&self) -> Result<()> {
        crate::request_exit(&self.app, true)
            .map_err(|error| windows::core::Error::new(E_FAIL, error))
    }
    fn WaitForInputIdle(&self, _milliseconds: i32) -> Result<BOOL> {
        Ok(true.into())
    }
    fn CanMaximize(&self) -> Result<BOOL> {
        Ok(false.into())
    }
    fn CanMinimize(&self) -> Result<BOOL> {
        Ok(false.into())
    }
    fn IsModal(&self) -> Result<BOOL> {
        Ok(false.into())
    }
    fn WindowVisualState(&self) -> Result<WindowVisualState> {
        Ok(WindowVisualState_Normal)
    }
    fn WindowInteractionState(&self) -> Result<WindowInteractionState> {
        Ok(WindowInteractionState_ReadyForUserInteraction)
    }
    fn IsTopmost(&self) -> Result<BOOL> {
        Ok(true.into())
    }
}
pub fn create(hwnd: isize, app: AppHandle) -> IRawElementProviderSimple {
    PlantProvider { hwnd, app }.into()
}
pub fn result(hwnd: isize, wp: usize, lp: isize, provider: &IRawElementProviderSimple) -> isize {
    unsafe { UiaReturnRawElementProvider(HWND(hwnd as *mut _), WPARAM(wp), LPARAM(lp), provider) }.0
}
pub fn disconnect(provider: &IRawElementProviderSimple) {
    let _ = unsafe { UiaDisconnectProvider(provider) };
}
pub const ROOT_OBJECT: isize = UiaRootObjectId as isize;
