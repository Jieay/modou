use serde::{Deserialize, Serialize};

// ====================================================================
// 几何换算（纯函数，跨平台，可单测）
// ====================================================================

/// 矩形，坐标为 macOS 全局逻辑点或 CSS px（取决于上下文）
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// 将前端 CSS px rect 换算为全局逻辑点。
/// - `css`: getBoundingClientRect() 得到的矩形（CSS px，相对 webview 内容区）
/// - `inner_x/inner_y`: Tauri window.inner_position() 物理像素坐标
/// - `scale`: window.scale_factor()
pub fn css_to_global(css: Rect, inner_x: i32, inner_y: i32, scale: f64) -> Rect {
    if scale == 0.0 {
        return css; // 防御性：避免除零
    }
    let logical_x = inner_x as f64 / scale;
    let logical_y = inner_y as f64 / scale;
    Rect {
        x: logical_x + css.x,
        y: logical_y + css.y,
        w: css.w,
        h: css.h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_1_identity_offset() {
        let css = Rect { x: 100.0, y: 200.0, w: 400.0, h: 600.0 };
        let g = css_to_global(css, 50, 60, 1.0);
        assert!((g.x - 150.0).abs() < 1e-9);
        assert!((g.y - 260.0).abs() < 1e-9);
        assert!((g.w - 400.0).abs() < 1e-9);
        assert!((g.h - 600.0).abs() < 1e-9);
    }

    #[test]
    fn scale_2_retina() {
        // 物理像素 200x120 在 scale=2 下对应逻辑点 100x60
        let css = Rect { x: 10.0, y: 20.0, w: 400.0, h: 600.0 };
        let g = css_to_global(css, 200, 120, 2.0);
        assert!((g.x - 110.0).abs() < 1e-9);
        assert!((g.y - 80.0).abs() < 1e-9);
        assert!((g.w - 400.0).abs() < 1e-9);
    }

    #[test]
    fn negative_coordinates_multimonitor() {
        let css = Rect { x: 0.0, y: 0.0, w: 500.0, h: 800.0 };
        let g = css_to_global(css, -2880, 0, 2.0); // 负坐标显示器
        assert!((g.x - (-1440.0)).abs() < 1e-9);
        assert!((g.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn zero_size_is_valid() {
        let css = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        let g = css_to_global(css, 0, 0, 2.0);
        assert_eq!(g, Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 });
    }
}

// ====================================================================
// macOS Accessibility (AX) 封装
// ====================================================================

#[cfg(target_os = "macos")]
pub mod ax {
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    /// AX 错误码
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(i32)]
    pub enum AxError {
        Success = 0,
        Failure = -25200,
        IllegalArgument = -25201,
        InvalidUIElement = -25202,
        InvalidUIElementObserver = -25203,
        CannotComplete = -25204,
        AttributeUnsupported = -25205,
        ActionUnsupported = -25206,
        NotificationUnsupported = -25208,
        NotImplemented = -25209,
        NotificationAlreadyRegistered = -25210,
        NotificationNotRegistered = -25211,
        APIDisabled = -25212,
        NoValue = -25213,
        ParameterizedAttributeUnsupported = -25214,
        NotEnoughPrecision = -25215,
        Other = -25999,
    }

    impl From<i32> for AxError {
        fn from(code: i32) -> Self {
            match code {
                0 => AxError::Success,
                -25200 => AxError::Failure,
                -25201 => AxError::IllegalArgument,
                -25202 => AxError::InvalidUIElement,
                -25203 => AxError::InvalidUIElementObserver,
                -25204 => AxError::CannotComplete,
                -25205 => AxError::AttributeUnsupported,
                -25206 => AxError::ActionUnsupported,
                -25208 => AxError::NotificationUnsupported,
                -25209 => AxError::NotImplemented,
                -25210 => AxError::NotificationAlreadyRegistered,
                -25211 => AxError::NotificationNotRegistered,
                -25212 => AxError::APIDisabled,
                -25213 => AxError::NoValue,
                -25214 => AxError::ParameterizedAttributeUnsupported,
                -25215 => AxError::NotEnoughPrecision,
                _ => AxError::Other,
            }
        }
    }

    /// AXValueType 枚举
    const AX_VALUE_CG_POINT_TYPE: u32 = 1;
    const AX_VALUE_CG_SIZE_TYPE: u32 = 2;

    /// CGPoint (逻辑点)
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    /// CGSize (逻辑点)
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    extern "C" {
        // HIServices / ApplicationServices
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;

        fn AXUIElementCreateApplication(pid: i32) -> *const std::ffi::c_void;
        fn AXUIElementCopyAttributeValue(
            element: *const std::ffi::c_void,
            attribute: CFTypeRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXUIElementSetAttributeValue(
            element: *const std::ffi::c_void,
            attribute: CFTypeRef,
            value: CFTypeRef,
        ) -> i32;
        fn AXUIElementPerformAction(
            element: *const std::ffi::c_void,
            action: CFTypeRef,
        ) -> i32;

        fn AXValueCreate(value_type: u32, value_ptr: *const std::ffi::c_void)
            -> *const std::ffi::c_void;
        fn AXValueGetValue(
            value: *const std::ffi::c_void,
            value_type: u32,
            value_ptr: *mut std::ffi::c_void,
        ) -> bool;
    }

    /// AX 窗口句柄，仅在 Rust 内存中持有，不可序列化
    #[derive(Debug)]
    pub struct AxWindowHandle {
        raw: *const std::ffi::c_void,
    }

    // AXUIElementRef 是线程安全的（Apple 文档）
    unsafe impl Send for AxWindowHandle {}
    unsafe impl Sync for AxWindowHandle {}

    impl AxWindowHandle {
        /// 从已有 AXUIElementRef 创建（获取所有权）
        pub fn from_raw(raw: *const std::ffi::c_void) -> Self {
            Self { raw }
        }

        pub fn raw(&self) -> *const std::ffi::c_void {
            self.raw
        }

        pub fn is_null(&self) -> bool {
            self.raw.is_null()
        }

        /// 读取 Position + Size，返回逻辑点 rect
        pub fn frame(&self) -> Option<crate::docking::Rect> {
            let pos = self.read_point("AXPosition")?;
            let size = self.read_size("AXSize")?;
            Some(crate::docking::Rect {
                x: pos.0,
                y: pos.1,
                w: size.0,
                h: size.1,
            })
        }

        /// 设置 Position + Size（逻辑点），应用前做容差判断避免抖动
        pub fn set_frame(&self, rect: crate::docking::Rect, tolerance: f64) -> bool {
            let current = self.frame();
            if let Some(cur) = current {
                let dx = (cur.x - rect.x).abs();
                let dy = (cur.y - rect.y).abs();
                let dw = (cur.w - rect.w).abs();
                let dh = (cur.h - rect.h).abs();
                if dx <= tolerance && dy <= tolerance && dw <= tolerance && dh <= tolerance {
                    return true; // 在容差内，不重设
                }
            }
            self.set_point("AXPosition", rect.x, rect.y);
            self.set_size("AXSize", rect.w, rect.h);
            true
        }

        /// 强制设置 frame，不做容差判断
        pub fn set_frame_force(&self, rect: crate::docking::Rect) {
            self.set_point("AXPosition", rect.x, rect.y);
            self.set_size("AXSize", rect.w, rect.h);
        }

        /// 读取标题
        pub fn title(&self) -> Option<String> {
            unsafe {
                let attr = CFString::new("AXTitle");
                let mut value: CFTypeRef = std::ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    self.raw,
                    attr.as_concrete_TypeRef() as *const std::ffi::c_void,
                    &mut value,
                );
                if err != 0 || value.is_null() {
                    return None;
                }
                let result = cfstring_to_string(value);
                CFRelease(value);
                result
            }
        }

        /// raise 窗口
        pub fn raise(&self) -> AxError {
            unsafe {
                let action = CFString::new("AXRaise");
                AxError::from(AXUIElementPerformAction(
                    self.raw,
                    action.as_concrete_TypeRef() as *const std::ffi::c_void,
                ))
            }
        }

        /// close 窗口（执行 AXClose action）
        pub fn close(&self) -> AxError {
            unsafe {
                let action = CFString::new("AXClose");
                AxError::from(AXUIElementPerformAction(
                    self.raw,
                    action.as_concrete_TypeRef() as *const std::ffi::c_void,
                ))
            }
        }

        fn read_point(&self, attr: &str) -> Option<(f64, f64)> {
            unsafe {
                let attr_cf = CFString::new(attr);
                let mut value: CFTypeRef = std::ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    self.raw,
                    attr_cf.as_concrete_TypeRef() as *const std::ffi::c_void,
                    &mut value,
                );
                if err != 0 || value.is_null() {
                    return None;
                }
                let mut point = CGPoint::default();
                let ok = AXValueGetValue(value, AX_VALUE_CG_POINT_TYPE, &mut point as *mut _ as *mut _);
                CFRelease(value);
                if !ok {
                    return None;
                }
                Some((point.x, point.y))
            }
        }

        fn read_size(&self, attr: &str) -> Option<(f64, f64)> {
            unsafe {
                let attr_cf = CFString::new(attr);
                let mut value: CFTypeRef = std::ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    self.raw,
                    attr_cf.as_concrete_TypeRef() as *const std::ffi::c_void,
                    &mut value,
                );
                if err != 0 || value.is_null() {
                    return None;
                }
                let mut size = CGSize::default();
                let ok = AXValueGetValue(value, AX_VALUE_CG_SIZE_TYPE, &mut size as *mut _ as *mut _);
                CFRelease(value);
                if !ok {
                    return None;
                }
                Some((size.width, size.height))
            }
        }

        fn set_point(&self, attr: &str, x: f64, y: f64) -> AxError {
            unsafe {
                let point = CGPoint { x, y };
                let value = AXValueCreate(
                    AX_VALUE_CG_POINT_TYPE,
                    &point as *const _ as *const _,
                );
                if value.is_null() {
                    return AxError::Failure;
                }
                let attr_cf = CFString::new(attr);
                let err = AXUIElementSetAttributeValue(
                    self.raw,
                    attr_cf.as_concrete_TypeRef() as *const std::ffi::c_void,
                    value,
                );
                CFRelease(value);
                AxError::from(err)
            }
        }

        fn set_size(&self, attr: &str, w: f64, h: f64) -> AxError {
            unsafe {
                let size = CGSize { width: w, height: h };
                let value = AXValueCreate(
                    AX_VALUE_CG_SIZE_TYPE,
                    &size as *const _ as *const _,
                );
                if value.is_null() {
                    return AxError::Failure;
                }
                let attr_cf = CFString::new(attr);
                let err = AXUIElementSetAttributeValue(
                    self.raw,
                    attr_cf.as_concrete_TypeRef() as *const std::ffi::c_void,
                    value,
                );
                CFRelease(value);
                AxError::from(err)
            }
        }
    }

    impl Clone for AxWindowHandle {
        fn clone(&self) -> Self {
            unsafe { CFRetain(self.raw) };
            Self { raw: self.raw }
        }
    }

    impl Drop for AxWindowHandle {
        fn drop(&mut self) {
            if !self.raw.is_null() {
                unsafe { CFRelease(self.raw) };
            }
        }
    }

    extern "C" {
        fn CFRetain(cf: CFTypeRef);
    }

    /// 检测当前进程是否已获辅助功能授权（只读，不弹窗）
    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// 请求辅助功能授权（弹一次系统提示），只能在按钮点击时调用
    pub fn request_trust() -> bool {
        unsafe {
           let key = CFString::new("AXTrustedCheckOptionPrompt");
           let value = CFNumber::from(1i32);
            let pairs = [(key, value)];
            let dict = CFDictionary::from_CFType_pairs(&pairs);
           AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as *const std::ffi::c_void)
        }
    }

    /// 为指定 PID 创建应用 AX 元素
    fn create_app_element(pid: u32) -> *const std::ffi::c_void {
        unsafe { AXUIElementCreateApplication(pid as i32) }
    }

    /// 获取应用的主窗口（优先 AXMainWindow，其次 AXWindows[0]）
    pub fn main_window_of(pid: u32) -> Option<AxWindowHandle> {
        unsafe {
            let app = create_app_element(pid);
            if app.is_null() {
                return None;
            }

            // 尝试 AXMainWindow
            let attr = CFString::new("AXMainWindow");
            let mut value: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(app, attr.as_concrete_TypeRef() as *const std::ffi::c_void, &mut value);
            if err == 0 && !value.is_null() {
                CFRelease(app);
                return Some(AxWindowHandle::from_raw(value));
            }

            // 回退到 AXWindows[0]
            let attr2 = CFString::new("AXWindows");
            let mut array_ref: CFTypeRef = std::ptr::null();
            let err2 = AXUIElementCopyAttributeValue(app, attr2.as_concrete_TypeRef() as *const std::ffi::c_void, &mut array_ref);
            CFRelease(app);
            if err2 == 0 && !array_ref.is_null() {
                // array_ref 是 CFArrayRef
               let arr = core_foundation::array::CFArray::<CFTypeRef>::wrap_under_create_rule(array_ref as *const _);
                if let Some(first) = arr.get(0) {
                    if !first.is_null() {
                        let win = *first;
                        unsafe { CFRetain(win) };
                        return Some(AxWindowHandle::from_raw(win));
                    }
                }
            }
            None
        }
    }

    /// 将 CFTypeRef (CFStringRef) 转为 Rust String
    fn cfstring_to_string(ref_: CFTypeRef) -> Option<String> {
        unsafe {
            // CFGetTypeID 比较 CFStringGetTypeID 来判断类型比较复杂
            // 直接尝试 wrap，如果不是 CFString 会 panic
            // 更安全的方法：用 CFString::wrap_under_get_rule，它能处理 CFStringRef
            // 但我们拿到的是 CFTypeRef，需要确保它确实是 CFString
            let cf_string = core_foundation::string::CFString::wrap_under_get_rule(ref_ as *const _);
            Some(cf_string.to_string())
        }
    }

    /// 用 mdfind 探测应用路径
    pub fn mdfind(bundle_id: &str) -> Option<String> {
        let query = format!("kMDItemCFBundleIdentifier == '{}'", bundle_id);
        let output = std::process::Command::new("mdfind")
            .arg(&query)
            .output()
            .ok()?;
        let path = String::from_utf8_lossy(&output.stdout);
        let path = path.lines().next()?.trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    /// 用 System Events 取进程 PID
    pub fn pid_of_process(app_name: &str) -> Option<u32> {
        let script = format!(
            "tell application \"System Events\" to unix id of first process whose name is \"{}\"",
            app_name
        );
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .ok()?;
        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            pid_str.parse::<u32>().ok()
        } else {
            None
        }
    }
}

// ====================================================================
// 非 macOS 桩实现
// ====================================================================

#[cfg(not(target_os = "macos"))]
pub mod ax {
    use super::Rect;

    pub struct AxWindowHandle;

    impl AxWindowHandle {
        pub fn is_null(&self) -> bool { true }
        pub fn frame(&self) -> Option<Rect> { None }
        pub fn set_frame(&self, _r: Rect, _t: f64) -> bool { false }
        pub fn set_frame_force(&self, _r: Rect) {}
        pub fn close(&self) -> i32 { -1 }
    }

    pub fn is_trusted() -> bool { false }
    pub fn request_trust() -> bool { false }
    pub fn main_window_of(_pid: u32) -> Option<AxWindowHandle> { None }
    pub fn mdfind(_bundle_id: &str) -> Option<String> { None }
    pub fn pid_of_process(_app_name: &str) -> Option<u32> { None }
}
