use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_millis(150);

pub async fn is_focused_editable() -> Option<bool> {
    tokio::time::timeout(PROBE_TIMEOUT, platform::probe())
        .await
        .ok()
        .flatten()
}

pub fn init() {
    platform::init();
}

#[cfg(target_os = "linux")]
mod platform {
    use atspi::{
        events::object::StateChangedEvent, proxy::accessible::AccessibleProxy, State,
    };
    use futures_util::StreamExt;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    static CACHE: OnceLock<Mutex<(Option<bool>, Instant)>> = OnceLock::new();

    fn cache() -> &'static Mutex<(Option<bool>, Instant)> {
        CACHE.get_or_init(|| Mutex::new((None, Instant::now())))
    }

    pub async fn probe() -> Option<bool> {
        let (val, when) = *cache().lock().ok()?;
        if when.elapsed() > std::time::Duration::from_secs(10) {
            None
        } else {
            val
        }
    }

    pub fn init() {
        tauri::async_runtime::spawn(async {
            if let Err(e) = listen().await {
                eprintln!("[polishd] atspi listener exited: {e}");
            }
        });
    }

    async fn listen() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = atspi::AccessibilityConnection::new().await?;
        conn.register_event::<StateChangedEvent>().await?;

        let mut stream = conn.event_stream();
        while let Some(Ok(ev)) = stream.next().await {
            let Ok(ev) = StateChangedEvent::try_from(ev) else { continue };
            if ev.enabled != 1 {
                continue;
            }
            if format!("{:?}", ev.state).to_lowercase() != "focused" {
                continue;
            }

            let editable = inspect(&conn, &ev.item).await.ok();
            if let Ok(mut guard) = cache().lock() {
                *guard = (editable, Instant::now());
            }
        }
        Ok(())
    }

    async fn inspect(
        conn: &atspi::AccessibilityConnection,
        item: &atspi::ObjectRef,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let proxy = AccessibleProxy::builder(conn.connection())
            .destination(item.name.as_str())?
            .path(item.path.as_str())?
            .build()
            .await?;

        let states = proxy.get_state().await?;
        if states.contains(State::Editable) {
            return Ok(true);
        }

        if let Ok(ifaces) = proxy.get_interfaces().await {
            if ifaces
                .iter()
                .any(|i| matches!(i, atspi::Interface::EditableText))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::{
        core::Interface,
        Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
        },
        Win32::UI::Accessibility::{
            CUIAutomation, IUIAutomation, IValueProvider, UIA_DocumentControlTypeId,
            UIA_EditControlTypeId, UIA_TextPatternId, UIA_ValuePatternId,
        },
    };

    pub fn init() {}

    pub async fn probe() -> Option<bool> {
        tokio::task::spawn_blocking(probe_blocking).await.ok()?
    }

    fn probe_blocking() -> Option<bool> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let focused = automation.GetFocusedElement().ok()?;

            if !focused.CurrentIsEnabled().ok()?.as_bool() {
                return Some(false);
            }

            let control_type = focused.CurrentControlType().ok()?.0;
            if control_type == UIA_EditControlTypeId.0
                || control_type == UIA_DocumentControlTypeId.0
            {
                if let Ok(pattern) = focused.GetCurrentPattern(UIA_ValuePatternId) {
                    if let Ok(value) = pattern.cast::<IValueProvider>() {
                        if let Ok(ro) = value.IsReadOnly() {
                            return Some(!ro.as_bool());
                        }
                    }
                }
                return Some(true);
            }

            if let Ok(pattern) = focused.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value) = pattern.cast::<IValueProvider>() {
                    if let Ok(ro) = value.IsReadOnly() {
                        return Some(!ro.as_bool());
                    }
                }
            }

            if focused.GetCurrentPattern(UIA_TextPatternId).is_ok() {
                return Some(false);
            }

            None
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use accessibility_sys::{
        kAXEnabledAttribute, kAXErrorSuccess, kAXFocusedUIElementAttribute,
        kAXRoleAttribute, kAXSelectedTextAttribute, AXIsProcessTrusted,
        AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
    };
    use core_foundation::{
        base::{CFRelease, TCFType},
        boolean::CFBoolean,
        string::{CFString, CFStringRef},
    };

    pub fn init() {}

    pub async fn probe() -> Option<bool> {
        tokio::task::spawn_blocking(probe_blocking).await.ok()?
    }

    fn probe_blocking() -> Option<bool> {
        unsafe {
            if !AXIsProcessTrusted() {
                return None;
            }

            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return None;
            }

            let focused = copy_attr(system, kAXFocusedUIElementAttribute)?;
            let focused_el = focused as AXUIElementRef;
            CFRelease(system as _);

            if let Some(enabled_raw) = copy_attr(focused_el, kAXEnabledAttribute) {
                let enabled: CFBoolean =
                    CFBoolean::wrap_under_create_rule(enabled_raw as _);
                if !bool::from(enabled) {
                    CFRelease(focused_el as _);
                    return Some(false);
                }
            }

            let role_raw = copy_attr(focused_el, kAXRoleAttribute)?;
            let role: CFString = CFString::wrap_under_create_rule(role_raw as CFStringRef);
            let role_str = role.to_string();

            let has_sel_attr = copy_attr(focused_el, kAXSelectedTextAttribute).is_some();

            CFRelease(focused_el as _);

            match role_str.as_str() {
                "AXTextField" | "AXTextArea" | "AXComboBox" => Some(true),
                "AXStaticText" | "AXLink" | "AXImage" | "AXHeading" => Some(false),
                _ => {
                    if has_sel_attr {
                        Some(true)
                    } else {
                        None
                    }
                }
            }
        }
    }

    unsafe fn copy_attr(
        el: AXUIElementRef,
        attr: &str,
    ) -> Option<*const std::ffi::c_void> {
        let key = CFString::new(attr);
        let mut value: *const std::ffi::c_void = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            el,
            key.as_concrete_TypeRef(),
            &mut value as *mut _ as *mut _,
        );
        if err == kAXErrorSuccess && !value.is_null() {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
    pub fn init() {}
    pub async fn probe() -> Option<bool> { None }
}
