//! 最近打开的项目：持久化存储、系统菜单、macOS Dock 菜单

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// 最近打开列表最大长度
const MAX_RECENT: usize = 10;

static RECENT_PROJECTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

fn storage_path(app: &AppHandle) -> PathBuf {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    config_dir.join("recent-projects.json")
}

/// 应用启动时加载最近记录（并缓存 AppHandle 供 Dock 菜单回调使用）
pub fn init(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
    if let Ok(content) = std::fs::read_to_string(storage_path(app)) {
        let list: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
        *RECENT_PROJECTS.lock().unwrap() = list;
    }
}

fn save(app: &AppHandle) {
    let list = RECENT_PROJECTS.lock().unwrap().clone();
    let path = storage_path(app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(path, content);
    }
}

/// 记录一个最近打开的项目并刷新菜单
pub fn add(app: &AppHandle, path: &str) {
    {
        let mut list = RECENT_PROJECTS.lock().unwrap();
        list.retain(|p| p != path);
        list.insert(0, path.to_string());
        list.truncate(MAX_RECENT);
    }
    save(app);
    let _ = build_menu(app);
}

/// 清空最近记录并刷新菜单
pub fn clear(app: &AppHandle) {
    RECENT_PROJECTS.lock().unwrap().clear();
    save(app);
    let _ = build_menu(app);
}

/// 按路径删除单条记录并刷新菜单
pub fn remove(app: &AppHandle, path: &str) {
    {
        let mut list = RECENT_PROJECTS.lock().unwrap();
        list.retain(|p| p != path);
    }
    save(app);
    let _ = build_menu(app);
}

fn list() -> Vec<String> {
    RECENT_PROJECTS.lock().unwrap().clone()
}

/// 路径显示名：取最后一段（如 modou）
fn display_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[tauri::command]
pub fn add_recent_project(path: String, app: AppHandle) {
    add(&app, &path);
}

#[tauri::command]
pub fn get_recent_projects() -> Vec<String> {
    list()
}

#[tauri::command]
pub fn remove_recent_project(path: String, app: AppHandle) {
    remove(&app, &path);
}

/// 构建系统菜单（全中文；默认菜单为英文故整体手工构建）
pub fn build_menu(app: &AppHandle) -> tauri::Result<()> {
    // 最近打开子菜单
    let mut recent_sub = SubmenuBuilder::new(app, "最近打开");
    let recents = list();
    if recents.is_empty() {
        recent_sub = recent_sub.item(
            &MenuItemBuilder::with_id("modou.recent.empty", "（无最近项目）")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for (i, p) in recents.iter().enumerate() {
            recent_sub = recent_sub.item(
                &MenuItemBuilder::with_id(format!("modou.recent.{i}"), display_name(p))
                    .build(app)?,
            );
        }
        recent_sub = recent_sub.separator().item(
            &MenuItemBuilder::with_id("modou.recent.clear", "清除全部记录").build(app)?,
        );
    }

    // App 菜单（macOS 上此菜单标题由系统显示为应用名）
    let app_menu = SubmenuBuilder::new(app, "墨斗")
        .item(&PredefinedMenuItem::about(app, Some("关于墨斗"), None)?)
        .separator()
        .item(&PredefinedMenuItem::services(app, Some("服务"))?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, Some("隐藏墨斗"))?)
        .item(&PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?)
        .item(&PredefinedMenuItem::show_all(app, Some("全部显示"))?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("退出墨斗"))?)
        .build()?;

    // 文件（⌘W 保留给网页内关闭标签，关闭窗口用 ⇧⌘W）
    let file_menu = SubmenuBuilder::new(app, "文件")
        .item(
            &MenuItemBuilder::with_id("modou.open_folder", "打开文件夹…")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("modou.new_window", "新建窗口")
                .accelerator("CmdOrCtrl+Shift+N")
                .build(app)?,
        )
        .separator()
        .item(&recent_sub.build()?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("modou.close_window", "关闭窗口")
                .accelerator("CmdOrCtrl+Shift+W")
                .build(app)?,
        )
        .build()?;

    // 编辑（预置项走原生 selector，网页/终端内复制粘贴依赖它们）
    let edit_menu = SubmenuBuilder::new(app, "编辑")
        .item(&PredefinedMenuItem::undo(app, Some("撤销"))?)
        .item(&PredefinedMenuItem::redo(app, Some("重做"))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some("剪切"))?)
        .item(&PredefinedMenuItem::copy(app, Some("复制"))?)
        .item(&PredefinedMenuItem::paste(app, Some("粘贴"))?)
        .item(&PredefinedMenuItem::select_all(app, Some("全选"))?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "视图")
        .item(&PredefinedMenuItem::fullscreen(app, Some("进入全屏"))?)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "窗口")
        .item(&PredefinedMenuItem::minimize(app, Some("最小化"))?)
        .item(&PredefinedMenuItem::maximize(app, Some("缩放"))?)
        .separator()
        .item(&PredefinedMenuItem::bring_all_to_front(
            app,
            Some("全部置于顶层"),
        )?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu])
        .build()?;
    app.set_menu(menu)?;
    Ok(())
}

/// 菜单事件分发（在 Builder::on_menu_event 中调用）
pub fn on_menu_event(app: &AppHandle, id: &str) {
    match id {
        "modou.open_folder" => {
            if let Some(w) = focused_window(app) {
                let _ = w.emit("menu:open-folder", ());
            }
        }
        "modou.new_window" => new_window(app),
        "modou.recent.clear" => clear(app),
        "modou.close_window" => {
            if let Some(w) = focused_window(app) {
                let _ = w.close();
            }
        }
        _ => {
            if let Some(rest) = id.strip_prefix("modou.recent.") {
                if let Ok(i) = rest.parse::<usize>() {
                    let path = list().get(i).cloned();
                    if let (Some(w), Some(p)) = (focused_window(app), path) {
                        let _ = w.emit("menu:open-project", p);
                    }
                }
            }
        }
    }
}

/// 当前获得焦点的窗口（无焦点窗口时回退到 main）
fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false))
        .or_else(|| app.get_webview_window("main"))
}

/// 在焦点窗口中打开指定路径的项目（菜单 / Dock 最近打开共用）
fn open_path_in_focused_window(app: &AppHandle, path: &str) {
    if let Some(w) = focused_window(app) {
        let _ = w.emit("menu:open-project", path.to_string());
    }
}

/// 新建窗口
pub fn new_window(app: &AppHandle) {
    let label = format!(
        "modou-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let _ = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("墨斗")
        .inner_size(1400.0, 900.0)
        .min_inner_size(900.0, 600.0)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .build();
}

/// macOS Dock 图标右键菜单。
///
/// Tauri 未暴露 Dock 菜单 API，这里子类化 tao 的 AppDelegate（保留其全部
/// 原有行为），附加 `applicationDockMenu:` 方法，每次右键时动态构建菜单。
#[cfg(target_os = "macos")]
pub mod dock {
    use super::{display_name, list, new_window, open_path_in_focused_window, APP_HANDLE};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, ProtocolObject, Sel};
    use objc2::{define_class, msg_send, sel, ClassType, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSMenu, NSMenuItem};
    use objc2_foundation::{NSObject, NSString};

    // Dock 菜单点击目标（菜单项 action 都转发到这里）
    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "ModouDockTarget"]
        struct ModouDockTarget;

        impl ModouDockTarget {
            #[unsafe(method(modouNewWindow:))]
            fn modou_new_window(&self, _sender: &NSMenuItem) {
                if let Some(app) = APP_HANDLE.get() {
                    new_window(app);
                }
            }

            #[unsafe(method(modouOpenRecent:))]
            fn modou_open_recent(&self, sender: &NSMenuItem) {
                let Some(obj) = sender.representedObject() else { return };
                let Ok(path) = obj.downcast::<NSString>() else { return };
                let Some(app) = APP_HANDLE.get() else { return };
                open_path_in_focused_window(app, &path.to_string());
            }
        }
    );

    // 以裸指针形式持有 target（菜单项的 target 是 weak 引用，需保证其常驻）
    static DOCK_TARGET_PTR: AtomicUsize = AtomicUsize::new(0);

    fn dock_target() -> &'static ModouDockTarget {
        let ptr = DOCK_TARGET_PTR.load(Ordering::Relaxed);
        unsafe { &*(ptr as *const ModouDockTarget) }
    }

    /// Dock 菜单被唤起时由 AppKit 调用，动态构建最新菜单
    extern "C" fn application_dock_menu(
        _this: *mut AnyObject,
        _sel: Sel,
        _sender: *mut NSApplication,
    ) -> *mut NSMenu {
        let Some(mtm) = MainThreadMarker::new() else {
            return std::ptr::null_mut();
        };
        let target = dock_target();

        let menu = NSMenu::new(mtm);

        let new_item = NSMenuItem::new(mtm);
        new_item.setTitle(&NSString::from_str("新建窗口"));
        unsafe {
            new_item.setAction(Some(sel!(modouNewWindow:)));
            new_item.setTarget(Some(target));
        }
        menu.addItem(&new_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let recents = list();
        if recents.is_empty() {
            let item = NSMenuItem::new(mtm);
            item.setTitle(&NSString::from_str("（无最近项目）"));
            item.setEnabled(false);
            menu.addItem(&item);
        } else {
            for p in recents {
                let item = NSMenuItem::new(mtm);
                item.setTitle(&NSString::from_str(&display_name(&p)));
                unsafe {
                    item.setAction(Some(sel!(modouOpenRecent:)));
                    item.setRepresentedObject(Some(&NSString::from_str(&p)));
                    item.setTarget(Some(target));
                }
                menu.addItem(&item);
            }
        }
        // 遵循 Cocoa 约定返回 autoreleased 对象
        Retained::autorelease_return(menu)
    }

    /// 安装 Dock 菜单：以当前 delegate 的类为父类生成子类并替换 delegate
    pub fn install() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let ns_app = NSApplication::sharedApplication(mtm);
        let Some(old_delegate) = ns_app.delegate() else { return };

        let superclass: &AnyClass = unsafe { msg_send![&*old_delegate, class] };
        let Some(mut builder) = ClassBuilder::new(c"ModouAppDelegate", superclass) else {
            return;
        };
        unsafe {
            builder.add_method(
                sel!(applicationDockMenu:),
                application_dock_menu
                    as extern "C" fn(*mut AnyObject, Sel, *mut NSApplication) -> *mut NSMenu,
            );
        }
        let cls = builder.register();

        let target: Retained<ModouDockTarget> = unsafe { msg_send![ModouDockTarget::class(), new] };
        let target: &'static ModouDockTarget = Box::leak(Box::new(target));
        DOCK_TARGET_PTR.store(target as *const _ as usize, Ordering::Relaxed);

        // delegate 属性为 weak，Box::leak 保证其实例常驻
        let delegate: Retained<AnyObject> = unsafe { msg_send![cls, new] };
        let delegate: &'static AnyObject = Box::leak(Box::new(delegate));
        unsafe {
            ns_app.setDelegate(Some(&*(delegate as *const AnyObject
                as *const ProtocolObject<dyn NSApplicationDelegate>)));
        }
    }
}
