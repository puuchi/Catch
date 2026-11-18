//! Windows boundary: one plain layered HWND, independent of WebView/Tao window
//! builders. No unstable Tauri APIs are used. All UI work stays on the UI thread.
use std::sync::atomic::{AtomicIsize, Ordering};
use tauri::{AppHandle, Manager};
pub struct Surface(AtomicIsize);

// One geometry contract for painting, HWND bounds, hit testing, and the main
// window's collapse/restore anchor. This is DPI scaling, not a user/global zoom.
const SURFACE_SIDE: i32 = 64;
pub fn surface_side(scale: f64) -> i32 {
    (f64::from(SURFACE_SIDE) * scale).round() as i32
}

#[cfg(windows)]
mod windows {
    use super::*;
    use catch_core::{growth::Plant, tracking::Status};
    use std::{
        cell::{Cell, RefCell},
        ffi::c_void,
        io::Cursor,
    };
    type WndProc = unsafe extern "system" fn(isize, u32, usize, isize) -> isize;
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct WndClass {
        style: u32,
        proc: Option<WndProc>,
        class_extra: i32,
        window_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu: *const u16,
        class: *const u16,
    }
    #[repr(C)]
    struct Monitor {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }
    #[repr(C)]
    struct MinMax {
        reserved: Point,
        max_size: Point,
        max_position: Point,
        min_track: Point,
        max_track: Point,
    }
    #[repr(C)]
    struct BitmapHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bits: u16,
        compression: u32,
        image_size: u32,
        x_pels: i32,
        y_pels: i32,
        used: u32,
        important: u32,
    }
    #[repr(C)]
    struct BitmapInfo {
        header: BitmapHeader,
        colors: [u32; 1],
    }
    #[repr(C)]
    struct Blend {
        op: u8,
        flags: u8,
        alpha: u8,
        format: u8,
    }
    type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;
    #[repr(C)]
    struct MeasureItem {
        control_type: u32,
        control_id: u32,
        item_id: u32,
        width: u32,
        height: u32,
        data: usize,
    }
    #[repr(C)]
    struct DrawItem {
        control_type: u32,
        control_id: u32,
        item_id: u32,
        action: u32,
        state: u32,
        item: isize,
        dc: isize,
        rect: Rect,
        data: usize,
    }
    #[repr(C)]
    struct MenuInfo {
        size: u32,
        mask: u32,
        style: u32,
        max_height: u32,
        background: isize,
        context_help: u32,
        data: usize,
    }
    #[repr(C)]
    struct MenuItemInfo {
        size: u32,
        mask: u32,
        item_type: u32,
        state: u32,
        id: u32,
        submenu: isize,
        checked: isize,
        unchecked: isize,
        data: usize,
        text: *mut u16,
        length: u32,
        bitmap: isize,
    }
    #[repr(C)]
    struct HighContrast {
        size: u32,
        flags: u32,
        scheme: *mut u16,
    }
    #[repr(C)]
    struct CallWndProc {
        lp: isize,
        wp: usize,
        message: u32,
        hwnd: isize,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(class: *const WndClass) -> u16;
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            param: *mut c_void,
        ) -> isize;
        fn DefWindowProcW(hwnd: isize, message: u32, wp: usize, lp: isize) -> isize;
        fn DestroyWindow(hwnd: isize) -> i32;
        fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
        fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
        fn GetDpiForWindow(hwnd: isize) -> u32;
        fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            after: isize,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn ShowWindow(hwnd: isize, command: i32) -> i32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn PostMessageW(hwnd: isize, message: u32, wp: usize, lp: isize) -> i32;
        fn LoadCursorW(instance: isize, name: *const u16) -> isize;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn SetCapture(hwnd: isize) -> isize;
        fn ReleaseCapture() -> i32;
        fn MonitorFromPoint(point: Point, flags: u32) -> isize;
        fn GetMonitorInfoW(monitor: isize, info: *mut Monitor) -> i32;
        fn UpdateLayeredWindow(
            hwnd: isize,
            destination: isize,
            position: *const Point,
            size: *const Point,
            source: isize,
            source_position: *const Point,
            key: u32,
            blend: *const Blend,
            flags: u32,
        ) -> i32;
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
        fn ValidateRect(hwnd: isize, rect: *const Rect) -> i32;
        fn CreatePopupMenu() -> isize;
        fn AppendMenuW(menu: isize, flags: u32, id: usize, text: *const u16) -> i32;
        fn TrackPopupMenu(
            menu: isize,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            hwnd: isize,
            rect: *const Rect,
        ) -> i32;
        fn DestroyMenu(menu: isize) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn SetWindowTextW(hwnd: isize, text: *const u16) -> i32;
        fn MessageBoxW(hwnd: isize, text: *const u16, title: *const u16, flags: u32) -> i32;
        fn GetClassNameW(hwnd: isize, class: *mut u16, capacity: i32) -> i32;
        fn SetWindowsHookExW(id: i32, hook: Option<HookProc>, module: isize, thread: u32) -> isize;
        fn UnhookWindowsHookEx(hook: isize) -> i32;
        fn CallNextHookEx(hook: isize, code: i32, wp: usize, lp: isize) -> isize;
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
        fn SetMenuInfo(menu: isize, info: *const MenuInfo) -> i32;
        fn SetMenuItemInfoW(
            menu: isize,
            item: u32,
            by_position: i32,
            info: *const MenuItemInfo,
        ) -> i32;
        fn SystemParametersInfoW(action: u32, param: u32, data: *mut c_void, flags: u32) -> i32;
        fn FillRect(dc: isize, rect: *const Rect, brush: isize) -> i32;
        fn DrawTextW(dc: isize, text: *const u16, count: i32, rect: *mut Rect, format: u32) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn DeleteDC(dc: isize) -> i32;
        fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut c_void,
            section: isize,
            offset: u32,
        ) -> isize;
        fn SelectObject(dc: isize, object: isize) -> isize;
        fn DeleteObject(object: isize) -> i32;
        fn CreateSolidBrush(color: u32) -> isize;
        fn CreateRoundRectRgn(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            width: i32,
            height: i32,
        ) -> isize;
        fn FillRgn(dc: isize, region: isize, brush: isize) -> i32;
        fn SetTextColor(dc: isize, color: u32) -> u32;
        fn SetBkMode(dc: isize, mode: i32) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike_out: u32,
            char_set: u32,
            output_precision: u32,
            clip_precision: u32,
            quality: u32,
            pitch_and_family: u32,
            face: *const u16,
        ) -> isize;
    }
    #[cfg(test)]
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetMenuItemInfoW(
            menu: isize,
            item: u32,
            by_position: i32,
            info: *mut MenuItemInfo,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> isize;
        fn GetCurrentThreadId() -> u32;
    }
    #[link(name = "wtsapi32")]
    unsafe extern "system" {
        fn WTSRegisterSessionNotification(hwnd: isize, flags: u32) -> i32;
        fn WTSUnRegisterSessionNotification(hwnd: isize) -> i32;
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: isize,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    const REDRAW: u32 = 0x8001;
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
    const fn rgb(red: u32, green: u32, blue: u32) -> u32 {
        red | (green << 8) | (blue << 16)
    }
    #[derive(Clone, Copy)]
    struct PopupTheme {
        background: u32,
        text: u32,
        hover: u32,
        separator: u32,
        dark: bool,
    }
    thread_local! {
        static POPUP_THEME: Cell<PopupTheme> = const { Cell::new(PopupTheme {
            background: rgb(249, 249, 249),
            text: rgb(26, 26, 26),
            hover: rgb(234, 234, 234),
            separator: rgb(218, 218, 218),
            dark: false,
        }) };
        static POPUP_REGION: Cell<(isize, i32, i32)> = const { Cell::new((0, 0, 0)) };
    }
    fn mix(background: u32, foreground: u32, percent: u32) -> u32 {
        fn channel(background: u32, foreground: u32, shift: u32, percent: u32) -> u32 {
            let background = (background >> shift) & 0xff;
            let foreground = (foreground >> shift) & 0xff;
            (background * (100 - percent) + foreground * percent + 50) / 100
        }
        rgb(
            channel(background, foreground, 0, percent),
            channel(background, foreground, 8, percent),
            channel(background, foreground, 16, percent),
        )
    }
    fn dip(value: i32, dpi: u32) -> i32 {
        (value * dpi.max(96) as i32 + 48) / 96
    }
    fn high_contrast() -> bool {
        let mut contrast = HighContrast {
            size: std::mem::size_of::<HighContrast>() as u32,
            flags: 0,
            scheme: std::ptr::null_mut(),
        };
        unsafe {
            SystemParametersInfoW(
                0x0042,
                contrast.size,
                &mut contrast as *mut HighContrast as *mut c_void,
                0,
            ) != 0
                && contrast.flags & 1 != 0
        }
    }
    fn uxtheme(ordinal: u16) -> isize {
        let module = unsafe { GetModuleHandleW(wide("uxtheme.dll").as_ptr()) };
        if module == 0 {
            0
        } else {
            unsafe { GetProcAddress(module, usize::from(ordinal) as *const u8) }
        }
    }
    fn should_apps_use_dark_mode() -> bool {
        let address = uxtheme(132);
        if address == 0 {
            return false;
        }
        let should_use: unsafe extern "system" fn() -> bool =
            unsafe { std::mem::transmute(address) };
        unsafe { should_use() }
    }
    fn allow_dark_mode_for_window(hwnd: isize, allow: bool) {
        let address = uxtheme(133);
        if address != 0 {
            let allow_for_window: unsafe extern "system" fn(isize, bool) -> bool =
                unsafe { std::mem::transmute(address) };
            unsafe {
                allow_for_window(hwnd, allow);
            }
        }
    }
    fn popup_theme() -> Option<PopupTheme> {
        // High-contrast colors belong to the user's accessibility settings, so
        // leave the complete menu to Windows instead of approximating them.
        if high_contrast() {
            return None;
        }
        if should_apps_use_dark_mode() {
            return Some(PopupTheme {
                background: rgb(44, 44, 44),
                text: rgb(255, 255, 255),
                hover: rgb(65, 65, 65),
                separator: rgb(82, 82, 82),
                dark: true,
            });
        }
        let background = rgb(249, 249, 249);
        let text = rgb(26, 26, 26);
        Some(PopupTheme {
            background,
            text,
            hover: mix(background, text, 6),
            separator: mix(background, text, 14),
            dark: false,
        })
    }
    fn popup_metrics(item_id: u32, dpi: u32) -> (u32, u32) {
        (
            dip(144, dpi) as u32,
            dip(if item_id == 0 { 9 } else { 36 }, dpi) as u32,
        )
    }
    fn popup_content_rect(item_id: u32, mut rect: Rect, dpi: u32) -> Rect {
        if item_id == 1 {
            rect.top += dip(4, dpi);
        } else if item_id == 2 {
            rect.bottom -= dip(4, dpi);
        }
        rect
    }
    fn measure_popup_item(hwnd: isize, lp: isize) -> bool {
        if lp == 0 {
            return false;
        }
        let item = unsafe { &mut *(lp as *mut MeasureItem) };
        if item.control_type != 1 {
            return false;
        }
        let (width, height) = popup_metrics(item.item_id, unsafe { GetDpiForWindow(hwnd) });
        item.width = width;
        item.height = height;
        true
    }
    fn fill_rect(dc: isize, rect: &Rect, color: u32) {
        let brush = unsafe { CreateSolidBrush(color) };
        if brush != 0 {
            unsafe {
                FillRect(dc, rect, brush);
                DeleteObject(brush);
            }
        }
    }
    fn draw_popup_item(hwnd: isize, lp: isize) -> bool {
        if lp == 0 {
            return false;
        }
        let item = unsafe { &mut *(lp as *mut DrawItem) };
        if item.control_type != 1 {
            return false;
        }
        let theme = POPUP_THEME.with(Cell::get);
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        fill_rect(item.dc, &item.rect, theme.background);
        let content = popup_content_rect(item.item_id, item.rect, dpi);
        if item.item_id == 0 {
            let height = dip(1, dpi);
            let top = (content.top + content.bottom - height) / 2;
            fill_rect(
                item.dc,
                &Rect {
                    left: content.left + dip(12, dpi),
                    top,
                    right: content.right - dip(12, dpi),
                    bottom: top + height,
                },
                theme.separator,
            );
            return true;
        }
        if item.state & 1 != 0 {
            let radius = dip(4, dpi);
            let region = unsafe {
                CreateRoundRectRgn(
                    content.left + dip(4, dpi),
                    content.top,
                    content.right - dip(4, dpi),
                    content.bottom,
                    radius * 2,
                    radius * 2,
                )
            };
            let brush = unsafe { CreateSolidBrush(theme.hover) };
            if region != 0 && brush != 0 {
                unsafe {
                    FillRgn(item.dc, region, brush);
                }
            }
            unsafe {
                DeleteObject(region);
                DeleteObject(brush);
            }
        }
        let text = match item.item_id {
            1 => "打开 Catch",
            2 => "退出",
            _ => return true,
        };
        let face = wide("Microsoft YaHei UI");
        let font = unsafe {
            CreateFontW(
                -dip(14, dpi),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            )
        };
        let old = if font != 0 {
            unsafe { SelectObject(item.dc, font) }
        } else {
            0
        };
        let mut text_rect = Rect {
            left: content.left + dip(16, dpi),
            top: content.top,
            right: content.right - dip(16, dpi),
            bottom: content.bottom,
        };
        unsafe {
            let old_background_mode = SetBkMode(item.dc, 1);
            let old_text_color = SetTextColor(item.dc, theme.text);
            DrawTextW(
                item.dc,
                wide(text).as_ptr(),
                -1,
                &mut text_rect,
                0x0004 | 0x0020 | 0x0800,
            );
            if old_background_mode != 0 {
                SetBkMode(item.dc, old_background_mode);
            }
            if old_text_color != u32::MAX {
                SetTextColor(item.dc, old_text_color);
            }
            if font != 0 {
                SelectObject(item.dc, old);
                DeleteObject(font);
            }
        }
        true
    }
    unsafe extern "system" fn popup_menu_hook(code: i32, wp: usize, lp: isize) -> isize {
        if code >= 0 && lp != 0 {
            let message = unsafe { &*(lp as *const CallWndProc) };
            if message.message == 0x0047 {
                let mut class = [0_u16; 7];
                let length = unsafe { GetClassNameW(message.hwnd, class.as_mut_ptr(), 7) };
                if length == 6 && class[..6] == [35, 51, 50, 55, 54, 56] {
                    let mut rect = Rect::default();
                    unsafe {
                        GetWindowRect(message.hwnd, &mut rect);
                    }
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;
                    let key = (message.hwnd, width, height);
                    if width > 0 && height > 0 && POPUP_REGION.with(Cell::get) != key {
                        POPUP_REGION.with(|region| region.set(key));
                        let radius = dip(8, unsafe { GetDpiForWindow(message.hwnd) });
                        let region = unsafe {
                            CreateRoundRectRgn(0, 0, width, height, radius * 2, radius * 2)
                        };
                        if region != 0 && unsafe { SetWindowRgn(message.hwnd, region, 1) } == 0 {
                            unsafe {
                                DeleteObject(region);
                            }
                        }
                        allow_dark_mode_for_window(message.hwnd, POPUP_THEME.with(Cell::get).dark);
                    }
                }
            }
        }
        unsafe { CallNextHookEx(0, code, wp, lp) }
    }
    fn set_popup_item_type(menu: isize, position: u32, item_type: u32) -> bool {
        let info = MenuItemInfo {
            size: std::mem::size_of::<MenuItemInfo>() as u32,
            mask: 0x0100,
            item_type,
            state: 0,
            id: 0,
            submenu: 0,
            checked: 0,
            unchecked: 0,
            data: 0,
            text: std::ptr::null_mut(),
            length: 0,
            bitmap: 0,
        };
        unsafe { SetMenuItemInfoW(menu, position, 1, &info) != 0 }
    }
    fn show_popup_menu(hwnd: isize, cursor: Point) -> i32 {
        let menu = unsafe { CreatePopupMenu() };
        if menu == 0 {
            return 0;
        }
        let theme = popup_theme();
        let mut background = 0;
        let mut hook = 0;
        unsafe {
            let appended = AppendMenuW(menu, 0, 1, wide("打开 Catch").as_ptr()) != 0
                && AppendMenuW(menu, 0x0800, 0, std::ptr::null()) != 0
                && AppendMenuW(menu, 0, 2, wide("退出").as_ptr()) != 0;
            if !appended {
                DestroyMenu(menu);
                return 0;
            }
            if let Some(theme) = theme.filter(|_| {
                // Change only the drawing type. Keep the menu strings for accessibility.
                if set_popup_item_type(menu, 0, 0x0100)
                    && set_popup_item_type(menu, 1, 0x0100 | 0x0800)
                    && set_popup_item_type(menu, 2, 0x0100)
                {
                    true
                } else {
                    for (position, item_type) in [(0, 0), (1, 0x0800), (2, 0)] {
                        set_popup_item_type(menu, position, item_type);
                    }
                    false
                }
            }) {
                POPUP_THEME.with(|current| current.set(theme));
                POPUP_REGION.with(|region| region.set((0, 0, 0)));
                background = CreateSolidBrush(theme.background);
                SetMenuInfo(
                    menu,
                    &MenuInfo {
                        size: std::mem::size_of::<MenuInfo>() as u32,
                        mask: 0x0002,
                        style: 0,
                        max_height: 0,
                        background,
                        context_help: 0,
                        data: 0,
                    },
                );
                hook = SetWindowsHookExW(4, Some(popup_menu_hook), 0, GetCurrentThreadId());
            }
            SetForegroundWindow(hwnd);
        }
        let selected = unsafe {
            TrackPopupMenu(
                menu,
                0x0100 | 0x0080,
                cursor.x,
                cursor.y,
                0,
                hwnd,
                std::ptr::null(),
            )
        };
        unsafe {
            if hook != 0 {
                UnhookWindowsHookEx(hook);
            }
            POPUP_REGION.with(|region| region.set((0, 0, 0)));
            DestroyMenu(menu);
            if background != 0 {
                DeleteObject(background);
            }
        }
        selected
    }
    struct Image {
        width: usize,
        height: usize,
        pixels: Vec<u8>,
    }
    impl Image {
        fn read(bytes: &[u8]) -> Result<Self, String> {
            let mut decoder = png::Decoder::new(Cursor::new(bytes));
            decoder
                .set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
            let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
            let mut buffer = vec![0; reader.output_buffer_size().ok_or("素材尺寸无效")?];
            let info = reader
                .next_frame(&mut buffer)
                .map_err(|error| error.to_string())?;
            buffer.truncate(info.buffer_size());
            let pixels = match info.color_type {
                png::ColorType::Rgba => buffer,
                png::ColorType::Rgb => buffer
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect(),
                _ => return Err("植物素材颜色格式无效".into()),
            };
            Ok(Self {
                width: info.width as usize,
                height: info.height as usize,
                pixels,
            })
        }
    }
    #[derive(Clone, Copy)]
    struct Drag {
        cursor: Point,
        origin: Point,
        moved: bool,
    }
    struct Context {
        app: AppHandle,
        crops: Image,
        sprout: Image,
        drag: Cell<Option<Drag>>,
        provider: Option<::windows::Win32::UI::Accessibility::IRawElementProviderSimple>,
        last_paint: RefCell<Option<PaintKey>>,
    }
    #[derive(PartialEq, Eq)]
    struct PaintKey {
        plant: Plant,
        stage: u8,
        status: Option<Status>,
        size: i32,
    }
    // The 64-DIP target includes a 4-DIP shadow gutter around a 56-DIP card.
    const ART_SIDE: usize = 44;
    fn physical_side(dpi: u32) -> i32 {
        surface_side(f64::from(dpi.max(96)) / 96.)
    }
    fn side(hwnd: isize) -> i32 {
        physical_side(unsafe { GetDpiForWindow(hwnd) })
    }
    fn card_distance(x: f64, y: f64) -> f64 {
        // Rounded rectangle: [4, 60] on both axes, with a 12-DIP radius.
        let qx = (x - 32.).abs() - 16.;
        let qy = (y - 32.).abs() - 16.;
        qx.max(0.).hypot(qy.max(0.)) + qx.max(qy).min(0.) - 12.
    }
    fn composite(pixel: &mut [u8], color: [u8; 3], alpha: u8) {
        // Source-over into the premultiplied BGRA required by UpdateLayeredWindow.
        let alpha = u32::from(alpha);
        for channel in 0..3 {
            pixel[channel] = ((u32::from(color[2 - channel]) * alpha
                + u32::from(pixel[channel]) * (255 - alpha))
                / 255) as u8;
        }
        pixel[3] = (alpha + u32::from(pixel[3]) * (255 - alpha) / 255) as u8;
    }
    fn card_pixels(size: usize) -> Vec<u8> {
        let scale = size as f64 / f64::from(SURFACE_SIDE);
        let mut pixels = vec![0_u8; size * size * 4];
        for y in 0..size {
            for x in 0..size {
                let px = (x as f64 + 0.5) / scale;
                let py = (y as f64 + 0.5) / scale;
                let pixel = &mut pixels[(y * size + x) * 4..][..4];
                // Retain the full square hit target, including the shadow gutter.
                pixel[3] = 1;
                let shadow = (1. - card_distance(px, py - 1.).max(0.) / 4.)
                    .clamp(0., 1.)
                    .powi(2);
                composite(pixel, [37, 51, 48], (shadow * 30.) as u8);
                let distance = card_distance(px, py);
                let coverage = (0.5 - distance * scale).clamp(0., 1.);
                let vertical = ((py - 4.) / 56.).clamp(0., 1.);
                // Deliberate software frost: tint, fine grain, rim and sheen.
                // This per-pixel-alpha WS_EX_LAYERED / UpdateLayeredWindow HWND
                // is not a supported DWM system-backdrop target. Do not apply an
                // undocumented accent policy or claim that desktop pixels blur.
                let grain = ((x * 13 + y * 7) % 5) as f64 - 2.;
                let color = [
                    (242. - vertical * 18. + grain) as u8,
                    (248. - vertical * 15. + grain) as u8,
                    (245. - vertical * 16. + grain) as u8,
                ];
                composite(pixel, color, (coverage * (218. - vertical * 18.)) as u8);
                let inner = (0.5 - (distance + 1.) * scale).clamp(0., 1.);
                let border = (coverage - inner).max(0.);
                composite(
                    pixel,
                    [255, 255, 255],
                    (border * (166. - vertical * 104.)) as u8,
                );
                let sheen = (1. - ((py - 5.) / 24.).clamp(0., 1.)).powi(2);
                composite(pixel, [255, 255, 255], (inner * sheen * 24.) as u8);
            }
        }
        pixels
    }
    fn work_area(point: Point) -> Rect {
        let mut info = Monitor {
            size: std::mem::size_of::<Monitor>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
        };
        unsafe { GetMonitorInfoW(MonitorFromPoint(point, 2), &mut info) };
        info.work
    }
    fn clamp(point: Point, width: i32, height: i32) -> Point {
        let work = work_area(point);
        Point {
            x: point
                .x
                .clamp(work.left, (work.right - width).max(work.left)),
            y: point
                .y
                .clamp(work.top, (work.bottom - height).max(work.top)),
        }
    }
    fn paint(hwnd: isize, context: &Context, force: bool) -> Result<(), String> {
        let (plant, stage, status) = crate::presentation(&context.app);
        let key = PaintKey {
            plant,
            stage,
            status: status.clone(),
            size: side(hwnd).max(1),
        };
        if !force && context.last_paint.borrow().as_ref() == Some(&key) {
            return Ok(());
        }
        let plant_name = match plant {
            Plant::Tulip => "郁金香",
            Plant::Sunflower => "向日葵",
            Plant::Corn => "玉米",
            Plant::Strawberry => "草莓",
            Plant::Sprout => "绿芽",
        };
        let status_name = match status {
            Some(Status::Running) => "正在投入",
            Some(Status::Paused) => "准备继续",
            _ => "今日成长",
        };
        unsafe {
            SetWindowTextW(
                hwnd,
                wide(&format!("Catch · {plant_name} · {status_name}")).as_ptr(),
            );
        }
        let stage = usize::from(stage.min(4));
        let (image, cell, column, row) = if plant == Plant::Sprout {
            (
                &context.sprout,
                context.sprout.height,
                [0, 8, 17, 26, 35][stage],
                0,
            )
        } else {
            (
                &context.crops,
                16,
                if matches!(plant, Plant::Corn | Plant::Strawberry) {
                    [5, 4, 3, 2, 1][stage]
                } else {
                    [11, 10, 9, 8, 7][stage]
                },
                match plant {
                    Plant::Tulip => 1,
                    Plant::Strawberry => 6,
                    _ => 9,
                },
            )
        };
        let size = key.size as usize;
        let mut pixels = card_pixels(size);
        let scale = size as f64 / f64::from(SURFACE_SIDE);
        // Nearest-neighbor sampling keeps the transparent pixel artwork crisp.
        let art_size = (size * ART_SIDE / SURFACE_SIDE as usize).max(1);
        let inset = (size - art_size) / 2;
        for y in 0..art_size {
            for x in 0..art_size {
                let source = ((row * cell + y * cell / art_size) * image.width
                    + column * cell
                    + x * cell / art_size)
                    * 4;
                let target = ((y + inset) * size + x + inset) * 4;
                composite(
                    &mut pixels[target..target + 4],
                    [
                        image.pixels[source],
                        image.pixels[source + 1],
                        image.pixels[source + 2],
                    ],
                    image.pixels[source + 3],
                );
            }
        }
        let color = match status {
            Some(Status::Running) => [81, 137, 76],
            Some(Status::Paused) => [183, 158, 98],
            _ => [156, 163, 175],
        };
        let radius = 4. * scale;
        for y in 0..size {
            for x in 0..size {
                let distance = ((x as f64 + 0.5 - 52. * scale).powi(2)
                    + (y as f64 + 0.5 - 51. * scale).powi(2))
                .sqrt();
                let pixel = &mut pixels[(y * size + x) * 4..][..4];
                let rim = (radius + scale + 0.5 - distance).clamp(0., 1.);
                composite(pixel, [255, 255, 250], (rim * 245.) as u8);
                let coverage = (radius + 0.5 - distance).clamp(0., 1.);
                composite(pixel, color, (coverage * 255.) as u8);
            }
        }
        let info = BitmapInfo {
            header: BitmapHeader {
                size: 40,
                width: size as i32,
                height: -(size as i32),
                planes: 1,
                bits: 32,
                compression: 0,
                image_size: pixels.len() as u32,
                x_pels: 0,
                y_pels: 0,
                used: 0,
                important: 0,
            },
            colors: [0],
        };
        let screen = unsafe { GetDC(0) };
        let memory = unsafe { CreateCompatibleDC(screen) };
        let mut bits = std::ptr::null_mut();
        let bitmap = unsafe { CreateDIBSection(screen, &info, 0, &mut bits, 0, 0) };
        if bitmap == 0 || memory == 0 || bits.is_null() {
            unsafe {
                if bitmap != 0 {
                    DeleteObject(bitmap);
                }
                if memory != 0 {
                    DeleteDC(memory);
                }
                ReleaseDC(0, screen);
            }
            return Err("植物位图创建失败".into());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
        }
        let old = unsafe { SelectObject(memory, bitmap) };
        let mut rect = Rect::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }
        let result = unsafe {
            UpdateLayeredWindow(
                hwnd,
                screen,
                &Point {
                    x: rect.left,
                    y: rect.top,
                },
                &Point {
                    x: size as i32,
                    y: size as i32,
                },
                memory,
                &Point::default(),
                0,
                &Blend {
                    op: 0,
                    flags: 0,
                    alpha: 255,
                    format: 1,
                },
                2,
            )
        };
        let failure = if result == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };
        unsafe {
            SelectObject(memory, old);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(0, screen);
        }
        if failure.is_none() {
            context.last_paint.replace(Some(key));
        }
        failure
            .map(|error| Err(format!("植物绘制失败：{error}")))
            .unwrap_or(Ok(()))
    }
    unsafe extern "system" fn callback(hwnd: isize, message: u32, wp: usize, lp: isize) -> isize {
        if message == 0x0081 {
            let context = unsafe { *(lp as *const *mut Context) };
            unsafe {
                SetWindowLongPtrW(hwnd, -21, context as isize);
            }
            return 1;
        }
        let pointer = unsafe { GetWindowLongPtrW(hwnd, -21) } as *mut Context;
        if pointer.is_null() {
            return unsafe { DefWindowProcW(hwnd, message, wp, lp) };
        }
        let context = unsafe { &*pointer };
        if message == 0x0082 {
            if let Some(provider) = &context.provider {
                crate::accessibility::disconnect(provider);
            }
            unsafe {
                WTSUnRegisterSessionNotification(hwnd);
                SetWindowLongPtrW(hwnd, -21, 0);
                drop(Box::from_raw(pointer));
            }
            return unsafe { DefWindowProcW(hwnd, message, wp, lp) };
        }
        match (message, wp) {
            (0x02b1, 7) => crate::interrupt(&context.app, "lock"),
            (0x0218, 4) => crate::interrupt(&context.app, "sleep"),
            (0x0011, _) | (0x0016, 1) => crate::interrupt(&context.app, "shutdown"),
            _ => {}
        }
        match message {
            0x003d if lp as i32 == crate::accessibility::ROOT_OBJECT as i32 => {
                if let Some(provider) = &context.provider {
                    return crate::accessibility::result(hwnd, wp, lp, provider);
                }
            }
            0x002c if measure_popup_item(hwnd, lp) => return 1,
            0x002b if draw_popup_item(hwnd, lp) => return 1,
            // Keep the standard window/caption accessibility semantics while
            // giving the layered surface the full client area, without chrome.
            0x0083 | 0x0085 => return 0,
            0x0086 => return 1,
            0x0084 => return 1,
            0x0011 => return 1,
            0x0010 => return 0,
            REDRAW | 0x000f => {
                if unsafe { IsWindowVisible(hwnd) } != 0 {
                    if let Err(error) = paint(hwnd, context, message == 0x000f) {
                        crate::report(&context.app, error);
                    }
                }
                unsafe {
                    ValidateRect(hwnd, std::ptr::null());
                }
                return 0;
            }
            0x0024 if lp != 0 => {
                let info = unsafe { &mut *(lp as *mut MinMax) };
                let side = side(hwnd);
                info.min_track = Point { x: side, y: side };
                info.max_track = Point { x: side, y: side };
                return 0;
            }
            0x02e0 | 0x007e => {
                let mut rect = Rect::default();
                unsafe {
                    GetWindowRect(hwnd, &mut rect);
                }
                // WM_DPICHANGED supplies destination-screen physical coordinates.
                // During a captured drag, keep the cursor-relative origin instead.
                if message == 0x02e0 && lp != 0 && context.drag.get().is_none() {
                    rect = unsafe { *(lp as *const Rect) };
                }
                let size = if message == 0x02e0 {
                    physical_side((wp & 0xffff) as u32)
                } else {
                    side(hwnd)
                };
                let point = clamp(
                    Point {
                        x: rect.left,
                        y: rect.top,
                    },
                    size,
                    size,
                );
                unsafe {
                    SetWindowPos(hwnd, -1, point.x, point.y, size, size, 0x0010);
                    PostMessageW(hwnd, REDRAW, 0, 0);
                }
                return 0;
            }
            0x0201 => {
                let mut cursor = Point::default();
                let mut rect = Rect::default();
                unsafe {
                    GetCursorPos(&mut cursor);
                    GetWindowRect(hwnd, &mut rect);
                    SetCapture(hwnd);
                }
                context.drag.set(Some(Drag {
                    cursor,
                    origin: Point {
                        x: rect.left,
                        y: rect.top,
                    },
                    moved: false,
                }));
                return 0;
            }
            0x0200 => {
                if let Some(mut drag) = context.drag.get() {
                    let mut cursor = Point::default();
                    unsafe {
                        GetCursorPos(&mut cursor);
                    }
                    let dx = cursor.x - drag.cursor.x;
                    let dy = cursor.y - drag.cursor.y;
                    if drag.moved || dx.abs() + dy.abs() >= (4 * side(hwnd) / SURFACE_SIDE).max(1) {
                        drag.moved = true;
                        let size = side(hwnd);
                        let point = clamp(
                            Point {
                                x: drag.origin.x + dx,
                                y: drag.origin.y + dy,
                            },
                            size,
                            size,
                        );
                        unsafe {
                            // A synchronous WM_DPICHANGED owns resizing; do not
                            // overwrite its new size with the source-screen size.
                            SetWindowPos(hwnd, -1, point.x, point.y, 0, 0, 0x0010 | 0x0001);
                        }
                        context.drag.set(Some(drag));
                    }
                }
                return 0;
            }
            0x0202 => {
                let drag = context.drag.take();
                unsafe {
                    ReleaseCapture();
                }
                if drag.is_some_and(|drag| !drag.moved) {
                    crate::expand(&context.app);
                } else if drag.is_some() {
                    let mut rect = Rect::default();
                    unsafe {
                        GetWindowRect(hwnd, &mut rect);
                    }
                    crate::remember_native_position(
                        &context.app,
                        tauri::PhysicalPosition::new(rect.left, rect.top),
                    );
                }
                return 0;
            }
            0x0215 => {
                context.drag.set(None);
                return 0;
            }
            0x0100 if wp == 13 || wp == 32 => {
                crate::expand(&context.app);
                return 0;
            }
            0x0205 => {
                let app = context.app.clone();
                let mut cursor = Point::default();
                unsafe {
                    GetCursorPos(&mut cursor);
                }
                let selected = show_popup_menu(hwnd, cursor);
                if selected == 1 {
                    crate::expand(&app);
                } else if selected == 2 {
                    if let Err(error) = crate::request_exit(&app, true) {
                        crate::report(&app, error);
                    }
                }
                return 0;
            }
            _ => {}
        }
        unsafe { DefWindowProcW(hwnd, message, wp, lp) }
    }
    pub fn install(app: &AppHandle) -> Result<isize, String> {
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class = wide("CatchNextNativePlant");
        let registration = WndClass {
            style: 0,
            proc: Some(callback),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: 0,
            cursor: unsafe { LoadCursorW(0, 32512 as *const u16) },
            background: 0,
            menu: std::ptr::null(),
            class: class.as_ptr(),
        };
        if unsafe { RegisterClassW(&registration) } == 0 {
            return Err(format!(
                "注册悬浮表面失败：{}",
                std::io::Error::last_os_error()
            ));
        }
        let context = Box::new(Context {
            app: app.clone(),
            crops: Image::read(include_bytes!(
                "../../frontend/public/assets/plants/Crop_Spritesheet.png"
            ))?,
            sprout: Image::read(include_bytes!(
                "../../frontend/public/assets/sprout/growing-plant-sprite.png"
            ))?,
            drag: Cell::new(None),
            provider: None,
            last_paint: RefCell::new(None),
        });
        let pointer = Box::into_raw(context);
        let hwnd = unsafe {
            CreateWindowExW(
                0x00080000 | 0x00000080 | 0x00000008,
                class.as_ptr(),
                wide("Catch · 今日植物").as_ptr(),
                0x80000000 | 0x00c00000 | 0x00080000,
                120,
                120,
                SURFACE_SIDE,
                SURFACE_SIDE,
                0,
                0,
                instance,
                pointer as *mut c_void,
            )
        };
        if hwnd == 0 {
            return Err(format!(
                "创建悬浮表面失败：{}",
                std::io::Error::last_os_error()
            ));
        }
        unsafe {
            (*pointer).provider = Some(crate::accessibility::create(hwnd, app.clone()));
        }
        if unsafe { WTSRegisterSessionNotification(hwnd, 0) } == 0 {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err("锁屏通知注册失败".into());
        }
        Ok(hwnd)
    }
    pub fn show(hwnd: isize, position: Option<tauri::PhysicalPosition<i32>>) -> Result<(), String> {
        let mut rect = Rect::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }
        let requested = position.map(|p| Point { x: p.x, y: p.y }).unwrap_or(Point {
            x: rect.left,
            y: rect.top,
        });
        // Move first so a restored position on another monitor establishes its
        // DPI before calculating the final size and work-area constraints.
        if unsafe { SetWindowPos(hwnd, -1, requested.x, requested.y, 0, 0, 0x0010 | 0x0001) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let size = side(hwnd);
        let point = clamp(requested, size, size);
        if unsafe { SetWindowPos(hwnd, -1, point.x, point.y, size, size, 0x0040 | 0x0010) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            PostMessageW(hwnd, REDRAW, 0, 0);
        }
        Ok(())
    }
    pub fn hide(hwnd: isize) {
        unsafe {
            ShowWindow(hwnd, 0);
        }
    }
    pub fn redraw(hwnd: isize) {
        unsafe {
            PostMessageW(hwnd, REDRAW, 0, 0);
        }
    }
    pub fn position(hwnd: isize) -> Option<tauri::PhysicalPosition<i32>> {
        if hwnd == 0 || unsafe { IsWindowVisible(hwnd) } == 0 {
            return None;
        }
        let mut rect = Rect::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return None;
        }
        Some(tauri::PhysicalPosition::new(rect.left, rect.top))
    }
    pub fn fit_main(
        window: &tauri::WebviewWindow,
        requested: Option<tauri::PhysicalPosition<i32>>,
    ) -> Result<(), String> {
        let current = window.outer_position().map_err(|error| error.to_string())?;
        let position = requested.unwrap_or(current);
        let work = work_area(Point {
            x: position.x,
            y: position.y,
        });
        let size = window.outer_size().map_err(|error| error.to_string())?;
        let width = (size.width as i32).min((work.right - work.left).max(1));
        let height = (size.height as i32).min((work.bottom - work.top).max(1));
        if size.width as i32 != width || size.height as i32 != height {
            window
                .set_size(tauri::PhysicalSize::new(width as u32, height as u32))
                .map_err(|error| error.to_string())?;
        }
        let point = clamp(
            Point {
                x: position.x,
                y: position.y,
            },
            width,
            height,
        );
        window
            .set_position(tauri::PhysicalPosition::new(point.x, point.y))
            .map_err(|error| error.to_string())
    }
    pub fn destroy(hwnd: isize) {
        unsafe {
            DestroyWindow(hwnd);
        }
    }
    pub fn open(target: &str) -> Result<(), String> {
        if target.contains('\0') {
            return Err("资料地址无效".into());
        }
        let result = unsafe {
            ShellExecuteW(
                0,
                wide("open").as_ptr(),
                wide(target).as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result <= 32 {
            Err(format!(
                "无法打开资料（系统返回 {result}），请检查地址或默认应用"
            ))
        } else {
            Ok(())
        }
    }
    pub fn confirm_quit() -> bool {
        unsafe {
            MessageBoxW(
                0,
                wide("正在计时，是否暂停并退出？").as_ptr(),
                wide("Catch").as_ptr(),
                0x00000004 | 0x00000100 | 0x00000020,
            ) == 6
        }
    }
    #[cfg(test)]
    mod surface_tests {
        use super::*;

        #[test]
        fn logical_size_tracks_monitor_dpi() {
            for (dpi, expected) in [(0, 64), (96, 64), (120, 80), (144, 96), (192, 128)] {
                assert_eq!(physical_side(dpi), expected);
                assert_eq!(
                    physical_side(dpi),
                    surface_side(f64::from(dpi.max(96)) / 96.)
                );
            }
        }

        #[test]
        fn silhouette_is_a_rounded_square() {
            assert!(card_distance(32., 32.) < 0.);
            assert_eq!(card_distance(4., 32.), 0.);
            assert_eq!(card_distance(60., 32.), 0.);
            assert!(card_distance(4., 4.) > 0.);
            assert!(card_distance(8., 32.) < 0.);
        }

        #[test]
        fn card_is_translucent_premultiplied_and_fully_hittable() {
            for size in [64, 80, 96, 128] {
                let pixels = card_pixels(size);
                assert_eq!(pixels.len(), size * size * 4);
                for pixel in pixels.as_chunks::<4>().0 {
                    assert!(pixel[3] > 0);
                    assert!(pixel[..3].iter().all(|channel| *channel <= pixel[3]));
                }
                let center = (size / 2 * size + size / 2) * 4;
                assert!((190..240).contains(&pixels[center + 3]));
                assert_eq!(pixels[3], 1);
            }
        }

        #[test]
        fn popup_metrics_scale_and_preserve_outer_padding() {
            assert_eq!(popup_metrics(1, 96), (144, 36));
            assert_eq!(popup_metrics(0, 96), (144, 9));
            assert_eq!(popup_metrics(1, 192), (288, 72));
            assert_eq!(dip(14, 144), 21);
            let rect = Rect {
                left: 0,
                top: 0,
                right: 144,
                bottom: 36,
            };
            assert_eq!(popup_content_rect(1, rect, 96).top, 4);
            assert_eq!(popup_content_rect(2, rect, 96).bottom, 32);
        }

        #[test]
        fn popup_theme_blending_preserves_color_channels() {
            let background = rgb(249, 250, 251);
            let text = rgb(1, 2, 3);
            assert_eq!(mix(background, text, 0), background);
            assert_eq!(mix(background, text, 100), text);
            assert_ne!(mix(background, text, 6), background);
        }

        #[test]
        fn popup_owner_draw_retains_system_menu_labels() {
            let menu = unsafe { CreatePopupMenu() };
            assert_ne!(menu, 0);
            let label = wide("打开 Catch");
            let appended = unsafe { AppendMenuW(menu, 0, 1, label.as_ptr()) } != 0;
            let styled = set_popup_item_type(menu, 0, 0x0100);
            let mut text = [0_u16; 32];
            let mut info = MenuItemInfo {
                size: std::mem::size_of::<MenuItemInfo>() as u32,
                mask: 0x0040 | 0x0100,
                item_type: 0,
                state: 0,
                id: 0,
                submenu: 0,
                checked: 0,
                unchecked: 0,
                data: 0,
                text: text.as_mut_ptr(),
                length: text.len() as u32,
                bitmap: 0,
            };
            let read = unsafe { GetMenuItemInfoW(menu, 0, 1, &mut info) } != 0;
            unsafe { DestroyMenu(menu) };
            assert!(appended && styled && read);
            assert_eq!(info.item_type, 0x0100);
            assert_eq!(text[..info.length as usize], label[..label.len() - 1]);
        }

        #[cfg(target_pointer_width = "64")]
        #[test]
        fn popup_win32_layouts_match_x64_abi() {
            assert_eq!(std::mem::size_of::<MeasureItem>(), 32);
            assert_eq!(std::mem::size_of::<DrawItem>(), 64);
            assert_eq!(std::mem::size_of::<MenuInfo>(), 40);
            assert_eq!(std::mem::size_of::<MenuItemInfo>(), 80);
            assert_eq!(std::mem::size_of::<HighContrast>(), 16);
            assert_eq!(std::mem::size_of::<CallWndProc>(), 32);
        }

        #[test]
        fn transparent_art_preserves_the_card() {
            let mut pixel = [30, 40, 50, 100];
            composite(&mut pixel, [255, 0, 0], 0);
            assert_eq!(pixel, [30, 40, 50, 100]);
            composite(&mut pixel, [81, 137, 76], 255);
            assert_eq!(pixel, [76, 137, 81, 255]);
        }
    }
}
fn hwnd(app: &AppHandle) -> isize {
    app.try_state::<Surface>()
        .map(|surface| surface.0.load(Ordering::SeqCst))
        .unwrap_or(0)
}
pub fn install(app: &AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        let handle = windows::install(app)?;
        app.manage(Surface(AtomicIsize::new(handle)));
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("当前桌面适配器只支持 Windows".into())
    }
}
pub fn show(app: &AppHandle, position: Option<tauri::PhysicalPosition<i32>>) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::show(hwnd(app), position)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, position);
        Err("当前桌面适配器只支持 Windows".into())
    }
}
pub fn hide(app: &AppHandle) {
    #[cfg(windows)]
    {
        let h = hwnd(app);
        let _ = app.run_on_main_thread(move || windows::hide(h));
    }
}
pub fn redraw(app: &AppHandle) {
    #[cfg(windows)]
    windows::redraw(hwnd(app));
}
pub fn position(app: &AppHandle) -> Option<tauri::PhysicalPosition<i32>> {
    #[cfg(windows)]
    {
        windows::position(hwnd(app))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        None
    }
}
pub fn fit_main(
    window: &tauri::WebviewWindow,
    requested: Option<tauri::PhysicalPosition<i32>>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::fit_main(window, requested)
    }
    #[cfg(not(windows))]
    {
        let _ = (window, requested);
        Ok(())
    }
}
pub fn destroy(app: &AppHandle) {
    #[cfg(windows)]
    {
        let h = app.state::<Surface>().0.swap(0, Ordering::SeqCst);
        let _ = app.run_on_main_thread(move || windows::destroy(h));
    }
}
pub fn open(target: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::open(target)
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        Err("当前桌面适配器只支持 Windows".into())
    }
}
pub fn confirm_quit() -> bool {
    #[cfg(windows)]
    {
        windows::confirm_quit()
    }
    #[cfg(not(windows))]
    {
        false
    }
}
