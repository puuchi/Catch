//! Windows boundary: a layered plant HWND and a native blur backdrop, independent of WebView/Tao window
//! builders. No unstable Tauri APIs are used. All UI work stays on the UI thread.
use std::sync::atomic::{AtomicIsize, Ordering};
use tauri::{AppHandle, Manager};
pub struct Surface(AtomicIsize);
pub(crate) const CARD_WIDTH: f64 = 68.;
pub(crate) const CARD_HEIGHT: f64 = 76.;
pub(crate) const MIN_SCALE: u16 = 75;
pub(crate) const MAX_SCALE: u16 = 150;

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
    #[derive(Default, Clone, Copy, PartialEq, Eq)]
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
    #[repr(C)]
    struct AccentPolicy {
        state: i32,
        flags: u32,
        color: u32,
        animation: u32,
    }
    #[repr(C)]
    struct CompositionData {
        attribute: i32,
        data: *mut c_void,
        size: usize,
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
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
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
        fn CreateRoundRectRgn(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            ellipse_width: i32,
            ellipse_height: i32,
        ) -> isize;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> *const c_void;
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
    fn enable_blur(hwnd: isize) -> bool {
        let module = unsafe { GetModuleHandleW(wide("user32.dll").as_ptr()) };
        let address =
            unsafe { GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr().cast()) };
        if address.is_null() {
            return false;
        }
        let set_composition = unsafe {
            std::mem::transmute::<
                *const c_void,
                unsafe extern "system" fn(isize, *mut CompositionData) -> i32,
            >(address)
        };
        let mut accent = AccentPolicy {
            state: 3,
            flags: 2,
            color: 0,
            animation: 0,
        };
        let mut data = CompositionData {
            attribute: 19,
            data: (&mut accent as *mut AccentPolicy).cast(),
            size: std::mem::size_of::<AccentPolicy>(),
        };
        unsafe { set_composition(hwnd, &mut data) != 0 }
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
        scale: Cell<u16>,
        wheel_delta: Cell<i32>,
        glass: Cell<isize>,
        glass_size: Cell<Point>,
    }
    #[derive(PartialEq, Eq)]
    struct PaintKey {
        plant: Plant,
        stage: u8,
        status: Option<Status>,
        size: Point,
    }
    fn size(hwnd: isize) -> Point {
        let pointer = unsafe { GetWindowLongPtrW(hwnd, -21) } as *const Context;
        let percent = if pointer.is_null() {
            100
        } else {
            unsafe { (*pointer).scale.get() }
        };
        let scale =
            f64::from(unsafe { GetDpiForWindow(hwnd) }.max(96)) / 96. * f64::from(percent) / 100.;
        Point {
            x: (CARD_WIDTH * scale).round() as i32,
            y: (CARD_HEIGHT * scale).round() as i32,
        }
    }
    fn place_glass(hwnd: isize, context: &Context, point: Point, size: Point, visible: bool) {
        let glass = context.glass.get();
        if glass == 0 {
            return;
        }
        if context.glass_size.get() != size {
            let scale = f64::from(size.x) / CARD_WIDTH;
            let inset = (2. * scale).round() as i32;
            let corner = (36. * scale).round() as i32;
            let region = unsafe {
                CreateRoundRectRgn(
                    inset,
                    inset,
                    size.x - inset + 1,
                    size.y - inset + 1,
                    corner,
                    corner,
                )
            };
            if region == 0 || unsafe { SetWindowRgn(glass, region, 0) } == 0 {
                unsafe {
                    if region != 0 {
                        DeleteObject(region);
                    }
                    ShowWindow(glass, 0);
                }
                return;
            }
            context.glass_size.set(size);
            context.last_paint.replace(None);
        }
        unsafe {
            SetWindowPos(
                glass,
                hwnd,
                point.x,
                point.y,
                size.x,
                size.y,
                0x0010 | if visible { 0x0040 } else { 0 },
            );
        }
    }
    fn resize(hwnd: isize, context: &Context, percent: u16) -> Result<(), String> {
        let percent = percent.clamp(MIN_SCALE, MAX_SCALE);
        if percent == context.scale.get() {
            return Ok(());
        }
        crate::save_native_scale(&context.app, percent)?;
        context.scale.set(percent);
        let mut rect = Rect::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }
        let size = size(hwnd);
        let point = clamp(
            Point {
                x: rect.left,
                y: rect.top,
            },
            size.x,
            size.y,
        );
        if unsafe { SetWindowPos(hwnd, -1, point.x, point.y, size.x, size.y, 0x0010) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        place_glass(hwnd, context, point, size, true);
        paint(hwnd, context, true)?;
        crate::remember_native_position(
            &context.app,
            tauri::PhysicalPosition::new(point.x, point.y),
        );
        Ok(())
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
    fn rounded_distance(x: f64, y: f64, half_width: f64, half_height: f64, radius: f64) -> f64 {
        let x = x.abs() - half_width + radius;
        let y = y.abs() - half_height + radius;
        x.max(0.).hypot(y.max(0.)) + x.max(y).min(0.) - radius
    }
    fn blend_pixel(pixels: &mut [u8], index: usize, color: [u8; 3], alpha: f64) {
        let alpha = alpha.clamp(0., 255.).round() as u32;
        for channel in 0..3 {
            pixels[index + channel] = ((u32::from(color[2 - channel]) * alpha
                + u32::from(pixels[index + channel]) * (255 - alpha))
                / 255) as u8;
        }
        pixels[index + 3] = (alpha + u32::from(pixels[index + 3]) * (255 - alpha) / 255) as u8;
    }
    fn paint(hwnd: isize, context: &Context, force: bool) -> Result<(), String> {
        let (plant, stage, status) = crate::presentation(&context.app);
        let key = PaintKey {
            plant,
            stage,
            status: status.clone(),
            size: size(hwnd),
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
        let dimensions = size(hwnd);
        let width = dimensions.x.max(1) as usize;
        let height = dimensions.y.max(1) as usize;
        let mut pixels = vec![0_u8; width * height * 4];
        let scale = width as f64 / CARD_WIDTH;
        for y in 0..height {
            for x in 0..width {
                let px = (x as f64 + 0.5) / scale;
                let py = (y as f64 + 0.5) / scale;
                let distance = rounded_distance(px - 34., py - 38., 32., 36., 18.) * scale;
                let target = (y * width + x) * 4;
                let shadow = 20. * (-distance.max(0.).powi(2) / (4. * scale * scale)).exp();
                blend_pixel(&mut pixels, target, [98, 97, 71], shadow);
                let coverage = (0.5 - distance).clamp(0., 1.);
                let glow = (-((px - 14.).powi(2) + (py - 10.).powi(2)) / 900.).exp();
                let opacity = if context.glass.get() != 0 && context.glass_size.get() == dimensions
                {
                    105.
                } else {
                    160.
                };
                blend_pixel(
                    &mut pixels,
                    target,
                    [253, 250, 238],
                    (opacity + glow * 35.) * coverage,
                );
                let rim = (distance + 1.3 * scale).max(0.) / (1.3 * scale);
                blend_pixel(
                    &mut pixels,
                    target,
                    [255, 255, 249],
                    215. * rim.min(1.) * coverage,
                );
                let platform = rounded_distance(px - 34., py - 51., 20., 2., 2.) * scale;
                blend_pixel(
                    &mut pixels,
                    target,
                    [210, 212, 187],
                    110. * (0.5 - platform).clamp(0., 1.),
                );
                pixels[target + 3] = pixels[target + 3].max(1);
            }
        }
        let art_size = (48. * scale).round().max(1.) as usize;
        let inset_x = (width - art_size) / 2;
        let inset_y = (5. * scale).round() as usize;
        for y in 0..art_size {
            for x in 0..art_size {
                let source = ((row * cell + y * cell / art_size) * image.width
                    + column * cell
                    + x * cell / art_size)
                    * 4;
                let target = ((y + inset_y) * width + x + inset_x) * 4;
                blend_pixel(
                    &mut pixels,
                    target,
                    [
                        image.pixels[source],
                        image.pixels[source + 1],
                        image.pixels[source + 2],
                    ],
                    f64::from(image.pixels[source + 3]),
                );
            }
        }
        let color = match status {
            Some(Status::Running) => [81, 137, 76],
            Some(Status::Paused) => [183, 158, 98],
            _ => [156, 163, 175],
        };
        for y in 0..height {
            for x in 0..width {
                let px = (x as f64 + 0.5) / scale;
                let py = (y as f64 + 0.5) / scale;
                let target = (y * width + x) * 4;
                let status_distance = (px - 55.).hypot(py - 45.);
                blend_pixel(
                    &mut pixels,
                    target,
                    [255, 255, 246],
                    230. * ((3. - status_distance) * scale).clamp(0., 1.),
                );
                blend_pixel(
                    &mut pixels,
                    target,
                    color,
                    255. * ((2.4 - status_distance) * scale).clamp(0., 1.),
                );
                for dot in 0..5 {
                    let current = dot == stage;
                    let dot_color = if current {
                        [64, 99, 73]
                    } else if dot < stage {
                        [145, 162, 126]
                    } else {
                        [194, 201, 175]
                    };
                    let radius = if current { 1.8 } else { 1.5 };
                    let distance = (px - (20. + dot as f64 * 7.)).hypot(py - 63.);
                    blend_pixel(
                        &mut pixels,
                        target,
                        dot_color,
                        255. * ((radius - distance) * scale + 0.5).clamp(0., 1.),
                    );
                }
            }
        }
        let info = BitmapInfo {
            header: BitmapHeader {
                size: 40,
                width: width as i32,
                height: -(height as i32),
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
                    x: width as i32,
                    y: height as i32,
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
                let dimensions = size(hwnd);
                info.min_track = dimensions;
                info.max_track = dimensions;
                return 0;
            }
            0x02e0 | 0x007e => {
                let mut rect = Rect::default();
                unsafe {
                    GetWindowRect(hwnd, &mut rect);
                }
                let size = size(hwnd);
                let point = clamp(
                    Point {
                        x: rect.left,
                        y: rect.top,
                    },
                    size.x,
                    size.y,
                );
                unsafe {
                    SetWindowPos(hwnd, -1, point.x, point.y, size.x, size.y, 0x0010);
                    PostMessageW(hwnd, REDRAW, 0, 0);
                }
                place_glass(
                    hwnd,
                    context,
                    point,
                    size,
                    unsafe { IsWindowVisible(hwnd) } != 0,
                );
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
                        let size = size(hwnd);
                        let point = clamp(
                            Point {
                                x: drag.origin.x + dx,
                                y: drag.origin.y + dy,
                            },
                            size.x,
                            size.y,
                        );
                        unsafe {
                            SetWindowPos(hwnd, -1, point.x, point.y, size.x, size.y, 0x0010);
                        }
                        place_glass(hwnd, context, point, size, true);
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
            0x020a if wp & 0x0008 != 0 => {
                let delta = i32::from((wp >> 16) as u16 as i16) + context.wheel_delta.get();
                let steps = delta / 120;
                context.wheel_delta.set(delta % 120);
                let percent = (i32::from(context.scale.get()) + steps * 5)
                    .clamp(i32::from(MIN_SCALE), i32::from(MAX_SCALE))
                    as u16;
                if let Err(error) = resize(hwnd, context, percent) {
                    crate::report(&context.app, error);
                }
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
                    AppendMenuW(
                        menu,
                        if context.scale.get() >= MAX_SCALE {
                            1
                        } else {
                            0
                        },
                        3,
                        wide("放大（Ctrl+滚轮）").as_ptr(),
                    );
                    AppendMenuW(
                        menu,
                        if context.scale.get() <= MIN_SCALE {
                            1
                        } else {
                            0
                        },
                        4,
                        wide("缩小（Ctrl+滚轮）").as_ptr(),
                    );
                    AppendMenuW(menu, 0, 5, wide("还原大小（68 × 76）").as_ptr());
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
                } else if (3..=5).contains(&selected) {
                    let percent = match selected {
                        3 => context.scale.get().saturating_add(5),
                        4 => context.scale.get().saturating_sub(5),
                        _ => 100,
                    };
                    if let Err(error) = resize(hwnd, context, percent) {
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
            scale: Cell::new(crate::native_scale(app)),
            wheel_delta: Cell::new(0),
            glass: Cell::new(0),
            glass_size: Cell::new(Point::default()),
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
                CARD_WIDTH as i32,
                CARD_HEIGHT as i32,
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
        // A separate compositor-backed HWND supplies the desktop blur. The
        // per-pixel layered plant stays above it and retains its own alpha.
        let glass = unsafe {
            CreateWindowExW(
                0x08000000 | 0x00000080 | 0x00000008,
                class.as_ptr(),
                wide("Catch · 玻璃背景").as_ptr(),
                0x80000000 | 0x08000000,
                120,
                120,
                CARD_WIDTH as i32,
                CARD_HEIGHT as i32,
                0,
                0,
                instance,
                std::ptr::null_mut(),
            )
        };
        if glass != 0 {
            if enable_blur(glass) {
                unsafe {
                    (*pointer).glass.set(glass);
                    SetWindowLongPtrW(hwnd, -8, glass);
                }
            } else {
                unsafe {
                    DestroyWindow(glass);
                }
            }
        }
        if unsafe { WTSRegisterSessionNotification(hwnd, 0) } == 0 {
            destroy(hwnd);
            return Err("锁屏通知注册失败".into());
        }
        Ok(hwnd)
    }
    pub fn show(hwnd: isize, position: Option<tauri::PhysicalPosition<i32>>) -> Result<(), String> {
        let mut rect = Rect::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }
        let size = size(hwnd);
        let point = clamp(
            position.map(|p| Point { x: p.x, y: p.y }).unwrap_or(Point {
                x: rect.left,
                y: rect.top,
            }),
            size.x,
            size.y,
        );
        if unsafe { SetWindowPos(hwnd, -1, point.x, point.y, size.x, size.y, 0x0040 | 0x0010) } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let pointer = unsafe { GetWindowLongPtrW(hwnd, -21) } as *const Context;
        if !pointer.is_null() {
            place_glass(hwnd, unsafe { &*pointer }, point, size, true);
        }
        unsafe {
            PostMessageW(hwnd, REDRAW, 0, 0);
        }
        Ok(())
    }
    pub fn hide(hwnd: isize) {
        let pointer = unsafe { GetWindowLongPtrW(hwnd, -21) } as *const Context;
        unsafe {
            ShowWindow(hwnd, 0);
            if !pointer.is_null() {
                ShowWindow((*pointer).glass.get(), 0);
            }
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
        let pointer = unsafe { GetWindowLongPtrW(hwnd, -21) } as *const Context;
        let glass = if pointer.is_null() {
            0
        } else {
            unsafe { (*pointer).glass.get() }
        };
        unsafe {
            DestroyWindow(hwnd);
            if glass != 0 {
                DestroyWindow(glass);
            }
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
