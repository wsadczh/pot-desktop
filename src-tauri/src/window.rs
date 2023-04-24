use crate::config::get_config;
use crate::selection::get_selection_text;
use crate::StringWrapper;
use crate::APP;
use tauri::{AppHandle, Manager, Window};
use toml::Value;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use window_shadows::set_shadow;

pub fn build_translate_window(
    label: &str,
    title: &str,
    handle: &AppHandle,
) -> Result<Window, String> {
    let (width, height) = get_window_size();
    let (x, y) = get_mouse_location().unwrap();
    let builder =
        tauri::WindowBuilder::new(handle, label, tauri::WindowUrl::App("index.html".into()))
            .inner_size(width, height)
            .always_on_top(true)
            .focused(true)
            .visible(false)
            .title(title);

    #[cfg(target_os = "macos")]
    {
        let builder = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true);
        let window = match label {
            "persistent" => builder.center().skip_taskbar(false).build().unwrap(),
            _ => builder.position(x, y).skip_taskbar(true).build().unwrap(),
        };
        set_shadow(&window, true).unwrap_or_default();
        Ok(window)
    }

    #[cfg(target_os = "windows")]
    {
        let builder = builder.decorations(false);
        let window = match label {
            "persistent" => builder.skip_taskbar(false).center().build().unwrap(),
            _ => builder.skip_taskbar(true).position(x, y).build().unwrap(),
        };
        set_shadow(&window, true).unwrap_or_default();
        Ok(window)
    }

    #[cfg(target_os = "linux")]
    {
        let builder = builder.transparent(true).decorations(false);
        let window = match label {
            "persistent" => builder.skip_taskbar(false).center().build().unwrap(),
            _ => builder.skip_taskbar(true).position(x, y).build().unwrap(),
        };
        Ok(window)
    }
}

pub fn build_ocr_window(handle: &AppHandle) -> Result<Window, String> {
    let window =
        tauri::WindowBuilder::new(handle, "ocr", tauri::WindowUrl::App("index.html".into()))
            .inner_size(800.0, 400.0)
            .min_inner_size(600.0, 400.0)
            .center()
            .focused(true)
            .title("OCR")
            .build()
            .unwrap();
    Ok(window)
}

// 获取默认窗口大小
fn get_window_size() -> (f64, f64) {
    let width: f64 = get_config("window_width", Value::from(400), APP.get().unwrap().state())
        .as_integer()
        .unwrap() as f64;
    let height: f64 = get_config(
        "window_height",
        Value::from(500),
        APP.get().unwrap().state(),
    )
    .as_integer()
    .unwrap() as f64;
    (width, height)
}

// 获取鼠标坐标
#[cfg(target_os = "linux")]
fn get_mouse_location() -> Result<(f64, f64), String> {
    use crate::config::get_monitor_info;
    use mouse_position::mouse_position::Mouse;

    let position = Mouse::get_mouse_position();
    let mut x = 0.0;
    let mut y = 0.0;

    let (width, height) = get_window_size();
    let handle = APP.get().unwrap();
    let (size_width, size_height, dpi) = get_monitor_info(handle.state());

    if let Mouse::Position { x: pos_x, y: pos_y } = position {
        x = pos_x as f64 / dpi;
        y = pos_y as f64 / dpi;
    }

    if x + width > size_width as f64 / dpi {
        x -= width;
        if x < 0.0 {
            x = 0.0;
        }
    }
    if y + height > size_height as f64 / dpi {
        y -= height;
        if y < 0.0 {
            y = 0.0;
        }
    }

    Ok((x, y))
}

#[cfg(target_os = "windows")]
fn get_mouse_location() -> Result<(f64, f64), String> {
    use crate::config::get_monitor_info;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let (width, height) = get_window_size();
    let handle = APP.get().unwrap();
    let (size_width, size_height, dpi) = get_monitor_info(handle.state());
    let mut point = POINT { x: 0, y: 0 };

    unsafe {
        if GetCursorPos(&mut point).as_bool() {
            let mut x = point.x as f64 / dpi;
            let mut y = point.y as f64 / dpi;
            // 由于获取到的屏幕大小以及鼠标坐标为物理像素，所以需要转换
            if x + width > size_width as f64 / dpi {
                x -= width;
                if x < 0.0 {
                    x = 0.0;
                }
            }
            if y + height > size_height as f64 / dpi {
                y -= height;
                if y < 0.0 {
                    y = 0.0;
                }
            }
            Ok((x, y))
        } else {
            Err("get cursorpos error".to_string())
        }
    }
}

#[cfg(target_os = "macos")]
fn get_mouse_location() -> Result<(f64, f64), String> {
    use core_graphics::display::CGDisplay;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let display = CGDisplay::main();
    let mode = display.display_mode().unwrap();
    let event =
        CGEvent::new(CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap());
    let point = event.unwrap().location();
    let mut x = point.x;
    let mut y = point.y;
    let (width, height) = get_window_size();
    if x + width > mode.width() as f64 {
        x = x - width;
        if x < 0.0 {
            x = 0.0;
        }
    }
    if y + height > mode.height() as f64 {
        y = y - height;
        if y < 0.0 {
            y = 0.0;
        }
    }
    return Ok((x, y));
}

// 划词翻译
pub fn translate_window() {
    // 获取选择文本
    let mut text = String::new();
    if let Ok(v) = get_selection_text() {
        text = v;
    }
    let handle = APP.get().unwrap();
    // 写入状态备用
    let state: tauri::State<StringWrapper> = handle.state();
    state.0.lock().unwrap().replace_range(.., &text);
    // 创建窗口
    match handle.get_window("translator") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_translate_window("translator", "Translator", handle).unwrap();
        }
    };
}

// 持久窗口
pub fn persistent_window() {
    let handle = APP.get().unwrap();
    match handle.get_window("persistent") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_translate_window("persistent", "Persistent", handle).unwrap();
        }
    };
}

// popclip划词翻译
pub fn popclip_window(text: String) {
    let handle = APP.get().unwrap();

    let state: tauri::State<StringWrapper> = handle.state();
    state.0.lock().unwrap().replace_range(.., &text);

    match handle.get_window("popclip") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_translate_window("popclip", "PopClip", handle).unwrap();
        }
    };
}

// OCR
#[allow(dead_code)]
pub fn ocr_window() {
    let handle = APP.get().unwrap();

    // 读取剪切板图片

    match handle.get_window("ocr") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _main_window = build_ocr_window(handle).unwrap();
        }
    };
}
