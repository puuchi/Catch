//! Windows boundary: one plain layered HWND, independent of WebView/Tao window
//! builders. No unstable Tauri APIs are used. All UI work stays on the UI thread.
use std::sync::atomic::{AtomicIsize, Ordering};
use tauri::{AppHandle, Manager};
pub struct Surface(AtomicIsize);

#[cfg(windows)]
mod windows {
    use super::*;
    use catch_core::{growth::Plant, tracking::Status};
    use std::{cell::Cell, ffi::c_void, io::Cursor};
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
        fn MessageBoxW(hwnd: isize, text: *const u16, title: *const u16, flags: u32) -> i32;
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
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
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
    }
    fn side(hwnd: isize) -> i32 {
        36 * unsafe { GetDpiForWindow(hwnd) }.max(96) as i32 / 96
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
    fn paint(hwnd: isize, context: &Context) -> Result<(), String> {
        let (plant, stage, status) = crate::presentation(&context.app);
        let stage = usize::from(stage.min(4));
        let (image, cell, column, row) = if plant == Plant::Sprout {
            (
                &context.sprout,
                context.sprout.height,
                [15, 20, 29, 42, 60][stage],
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
        let size = side(hwnd).max(1) as usize;
        let mut pixels = vec![0_u8; size * size * 4];
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 1;
        }
        for y in 0..size {
            for x in 0..size {
                let source = ((row * cell + y * cell / size) * image.width
                    + column * cell
                    + x * cell / size)
                    * 4;
                let target = (y * size + x) * 4;
                let alpha = u32::from(image.pixels[source + 3]);
                pixels[target] = (u32::from(image.pixels[source + 2]) * alpha / 255) as u8;
                pixels[target + 1] = (u32::from(image.pixels[source + 1]) * alpha / 255) as u8;
                pixels[target + 2] = (u32::from(image.pixels[source]) * alpha / 255) as u8;
                pixels[target + 3] = alpha.max(1) as u8;
            }
        }
        let color = match status {
            Some(Status::Running) => [81, 137, 76],
            Some(Status::Paused) => [183, 158, 98],
            _ => [156, 163, 175],
        };
        let scale = size as f64 / 36.;
        let radius = 3.5 * scale;
        for y in 0..size {
            for x in 0..size {
                let distance =
                    ((x as f64 - 31. * scale).powi(2) + (y as f64 - 31. * scale).powi(2)).sqrt();
                if distance <= radius {
                    let color = if distance > radius - scale {
                        [250, 250, 245]
                    } else {
                        color
                    };
                    pixels[(y * size + x) * 4..(y * size + x) * 4 + 4]
                        .copy_from_slice(&[color[2], color[1], color[0], 255]);
                }
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
            0x0011 => return 1,
            0x0010 => return 0,
            REDRAW | 0x000f => {
                if unsafe { IsWindowVisible(hwnd) } != 0 {
                    if let Err(error) = paint(hwnd, context) {
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
                let size = side(hwnd);
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
                    if drag.moved || dx.abs() + dy.abs() >= 4 {
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
                            SetWindowPos(hwnd, -1, point.x, point.y, size, size, 0x0010);
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
                let menu = unsafe { CreatePopupMenu() };
                unsafe {
                    AppendMenuW(menu, 0, 1, wide("打开 Catch").as_ptr());
                    AppendMenuW(menu, 0x0800, 0, std::ptr::null());
                    AppendMenuW(menu, 0, 2, wide("退出").as_ptr());
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
                    DestroyMenu(menu);
                }
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
        });
        let pointer = Box::into_raw(context);
        let hwnd = unsafe {
            CreateWindowExW(
                0x00080000 | 0x00000080 | 0x00000008,
                class.as_ptr(),
                wide("Catch · 今日植物").as_ptr(),
                0x80000000,
                120,
                120,
                36,
                36,
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
        let size = side(hwnd);
        let point = clamp(
            position.map(|p| Point { x: p.x, y: p.y }).unwrap_or(Point {
                x: rect.left,
                y: rect.top,
            }),
            size,
            size,
        );
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
