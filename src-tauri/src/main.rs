#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod clock;
mod native;
use catch_app::{
    view_session::{Phase, Session, UiState},
    Context, Request, Service,
};
use catch_core::{
    growth::DayBounds,
    resources::{Kind, Owner},
};
use catch_storage::{Disk, FileDisk, ProfileLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
static EXITING: AtomicBool = AtomicBool::new(false);
type Runtime = Mutex<Desktop>;
struct Desktop {
    service: Service,
    view: Session,
    disk: Arc<dyn Disk>,
    preferences: Preferences,
    hwnd: isize,
    day: String,
    _lock: ProfileLock,
    isolated: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Preferences {
    transparency: u8,
    pinned: bool,
    width: f64,
    height: f64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            transparency: 45,
            pinned: true,
            width: 320.,
            height: 510.,
        }
    }
}
fn ensure(window: &WebviewWindow, desktop: &Desktop, generation: u64) -> Result<(), String> {
    if window.label() != "main"
        || window.hwnd().map_err(|error| error.to_string())?.0 as isize != desktop.hwnd
    {
        return Err("窗口无此权限".into());
    }
    desktop.view.ensure(generation)
}
fn growth_view(desktop: &Desktop, now: i64) -> Result<catch_core::growth::View, String> {
    let day = clock::day(now);
    let (start, end) = clock::bounds(&day)?;
    desktop
        .service
        .growth_view(
            DayBounds {
                day: &day,
                start,
                end,
            },
            now,
        )
        .map_err(|error| error.message)
}
#[tauri::command]
fn boot(window: WebviewWindow, state: State<Runtime>) -> Result<Value, String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    let generation = desktop.view.generation();
    let registered = window
        .app_handle()
        .get_webview_window("main")
        .ok_or("窗口已更新")?;
    if desktop.view.phase() == Phase::Opening
        && registered.hwnd().map_err(|error| error.to_string())?
            == window.hwnd().map_err(|error| error.to_string())?
    {
        desktop.hwnd = window.hwnd().map_err(|error| error.to_string())?.0 as isize;
    }
    ensure(&window, &desktop, generation)?;
    let now = clock::now();
    Ok(
        json!({"generation":generation,"state":desktop.service.snapshot(),"ui":desktop.view.saved(),"now":now,"growth":growth_view(&desktop,now)?,"preferences":desktop.preferences,"isolated":desktop.isolated}),
    )
}
#[tauri::command]
fn execute(
    window: WebviewWindow,
    app: AppHandle,
    state: State<Runtime>,
    generation: u64,
    request: Request,
) -> Result<Value, String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    let now = clock::now();
    let day = clock::day(now);
    let id = uuid::Uuid::new_v4().to_string();
    let result = desktop.service.execute(
        request,
        Context {
            now,
            local_day: &day,
            new_id: &id,
        },
    );
    let snapshot = desktop.service.snapshot();
    let _ = app.emit_to("main", "catch:state", &snapshot);
    drop(desktop);
    native::redraw(&app);
    Ok(match result {
        Ok(_) => json!({"ok":true,"state":snapshot}),
        Err(error) => json!({"ok":false,"state":snapshot,"error":error}),
    })
}
#[tauri::command]
fn stash_ui(
    window: WebviewWindow,
    state: State<Runtime>,
    generation: u64,
    ui: UiState,
) -> Result<(), String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    desktop.view.stash(generation, ui)
}
#[tauri::command]
fn view_painted(
    window: WebviewWindow,
    app: AppHandle,
    state: State<Runtime>,
    generation: u64,
) -> Result<(), String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    if !desktop.view.painted(generation) {
        return Err("窗口绘制回调已过期".into());
    }
    drop(desktop);
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    native::hide(&app);
    diagnostics(&app);
    Ok(())
}
#[tauri::command]
fn collapse(
    window: WebviewWindow,
    app: AppHandle,
    state: State<Runtime>,
    generation: u64,
    ui: UiState,
) -> Result<(), String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    desktop.preferences.width = f64::from(size.width) / scale;
    desktop.preferences.height = f64::from(size.height) / scale;
    desktop.disk.write_atomic(
        "catch-preferences.json",
        &serde_json::to_vec(&desktop.preferences).map_err(|error| error.to_string())?,
    )?;
    desktop.view.prepare_collapse(generation, ui)?;
    drop(desktop);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let bubble_position = tauri::PhysicalPosition::new(
            position.x + (size.width as i32 - (36. * scale) as i32) / 2,
            position.y,
        );
        if let Err(error) = native::show(&handle, Some(bubble_position)) {
            if let Ok(mut desktop) = handle.state::<Runtime>().lock() {
                desktop.view.abort_collapse(generation);
            }
            report(&handle, error);
            return;
        }
        if let Err(error) = window.destroy() {
            if let Ok(mut desktop) = handle.state::<Runtime>().lock() {
                desktop.view.abort_collapse(generation);
            }
            native::hide(&handle);
            report(&handle, error.to_string());
        } else if let Ok(mut desktop) = handle.state::<Runtime>().lock() {
            desktop.view.finish_collapse(generation);
            desktop.hwnd = 0;
        }
        diagnostics(&handle);
    })
    .map_err(|error| error.to_string())
}
#[tauri::command]
fn save_preferences(
    window: WebviewWindow,
    state: State<Runtime>,
    generation: u64,
    preferences: Value,
) -> Result<(), String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    let transparency = preferences["transparency"]
        .as_u64()
        .filter(|value| *value <= 100)
        .ok_or("透明度无效")? as u8;
    let pinned = preferences["pinned"].as_bool().ok_or("置顶设置无效")?;
    let next = Preferences {
        transparency,
        pinned,
        ..desktop.preferences.clone()
    };
    desktop.disk.write_atomic(
        "catch-preferences.json",
        &serde_json::to_vec(&next).map_err(|error| error.to_string())?,
    )?;
    desktop.preferences = next;
    drop(desktop);
    window
        .set_always_on_top(pinned)
        .map_err(|error| error.to_string())
}
#[tauri::command]
async fn pick_resources(
    window: WebviewWindow,
    state: State<'_, Runtime>,
    generation: u64,
    kind: Kind,
) -> Result<Vec<String>, String> {
    {
        let desktop = state.lock().map_err(|_| "状态锁失败")?;
        ensure(&window, &desktop, generation)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let picker =
            rfd::FileDialog::new()
                .set_parent(&window)
                .set_title(if kind == Kind::Folder {
                    "关联文件夹"
                } else {
                    "关联文件"
                });
        let paths = if kind == Kind::Folder {
            picker.pick_folder().map(|path| vec![path])
        } else {
            picker.pick_files()
        };
        Ok(paths
            .unwrap_or_default()
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect())
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
async fn classify_resource(
    window: WebviewWindow,
    state: State<'_, Runtime>,
    generation: u64,
    path: String,
) -> Result<Kind, String> {
    {
        let desktop = state.lock().map_err(|_| "状态锁失败")?;
        ensure(&window, &desktop, generation)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::metadata(path)
            .map(|meta| {
                if meta.is_dir() {
                    Kind::Folder
                } else {
                    Kind::File
                }
            })
            .map_err(|_| "找不到这份资料".into())
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
async fn open_resource(
    window: WebviewWindow,
    state: State<'_, Runtime>,
    generation: u64,
    owner: Owner,
    id: String,
) -> Result<(), String> {
    let resource = {
        let desktop = state.lock().map_err(|_| "状态锁失败")?;
        ensure(&window, &desktop, generation)?;
        desktop
            .service
            .resource_for_open(&owner, &id)
            .map_err(|error| error.message)?
    };
    tauri::async_runtime::spawn_blocking(move || {
        if resource.kind != Kind::Web && !std::path::Path::new(&resource.target).exists() {
            return Err("找不到这份资料，可以在操作菜单中重新定位".into());
        }
        native::open(&resource.target)
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
fn quit(
    window: WebviewWindow,
    app: AppHandle,
    state: State<Runtime>,
    generation: u64,
) -> Result<(), String> {
    {
        let desktop = state.lock().map_err(|_| "状态锁失败")?;
        ensure(&window, &desktop, generation)?;
    }
    request_exit(&app, false)
}
pub(crate) fn report(app: &AppHandle, error: String) {
    let _ = app.emit_to("main", "catch:error", &error);
    eprintln!("{error}");
}
pub(crate) fn interrupt(app: &AppHandle, reason: &str) {
    let result = app
        .state::<Runtime>()
        .lock()
        .map_err(|_| "状态锁失败".to_string())
        .and_then(|mut desktop| {
            desktop
                .service
                .interrupt(clock::now(), reason)
                .map_err(|error| error.message)
        });
    if let Err(error) = result {
        report(app, error);
    }
    emit_state(app);
    native::redraw(app);
}
fn emit_state(app: &AppHandle) {
    if let Ok(desktop) = app.state::<Runtime>().lock() {
        let _ = app.emit_to("main", "catch:state", desktop.service.snapshot());
    }
}
pub(crate) fn presentation(
    app: &AppHandle,
) -> (
    catch_core::growth::Plant,
    u8,
    Option<catch_core::tracking::Status>,
) {
    let desktop = app.state::<Runtime>();
    let Ok(desktop) = desktop.lock() else {
        return (Default::default(), 0, None);
    };
    let snapshot = desktop.service.snapshot();
    let status = snapshot
        .tracking
        .current()
        .map(|matter| matter.status.clone())
        .or_else(|| {
            snapshot
                .tracking
                .activities
                .iter()
                .find(|matter| matter.status == catch_core::tracking::Status::Paused)
                .map(|matter| matter.status.clone())
        });
    let growth = growth_view(&desktop, clock::now()).ok();
    (
        growth.as_ref().map(|view| view.plant).unwrap_or_default(),
        growth.map(|view| view.stage).unwrap_or(0),
        status,
    )
}
pub(crate) fn request_exit(app: &AppHandle, confirm: bool) -> Result<(), String> {
    if confirm && app.get_webview_window("main").is_some() {
        let _ = app.emit_to("main", "catch:request-quit", ());
        return Ok(());
    }
    let running = app
        .state::<Runtime>()
        .lock()
        .map_err(|_| "状态锁失败")?
        .service
        .snapshot()
        .tracking
        .current()
        .is_some();
    if confirm && running && !native::confirm_quit() {
        return Ok(());
    }
    let state = app.state::<Runtime>();
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    desktop
        .service
        .interrupt(clock::now(), "quit")
        .map_err(|error| error.message)?;
    desktop
        .service
        .checkpoint(clock::now())
        .map_err(|error| error.message)?;
    drop(desktop);
    EXITING.store(true, Ordering::SeqCst);
    native::destroy(app);
    app.exit(0);
    Ok(())
}
pub(crate) fn expand(app: &AppHandle) {
    let (generation, preferences) = {
        let state = app.state::<Runtime>();
        let Ok(mut desktop) = state.lock() else {
            return;
        };
        match desktop.view.begin_open() {
            Ok(Some(generation)) => (generation, desktop.preferences.clone()),
            Ok(None) => {
                drop(desktop);
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                    let _ = main.set_focus();
                }
                return;
            }
            Err(error) => {
                report(app, error);
                return;
            }
        }
    };
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut builder = tauri::WebviewWindowBuilder::new(
            &handle,
            "main",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Catch · 本地预览")
        .inner_size(preferences.width, preferences.height)
        .min_inner_size(260., 300.)
        .decorations(false)
        .transparent(true)
        .visible(false)
        .always_on_top(preferences.pinned)
        .devtools(false)
        .on_navigation(|url| {
            url.host_str() == Some("tauri.localhost")
                || (cfg!(debug_assertions)
                    && url.host_str() == Some("127.0.0.1")
                    && url.port() == Some(1431))
        });
        builder = builder.center();
        match builder.build() {
            Ok(main) => {
                if let Some(position) = native::position(&handle) {
                    let scale = main.scale_factor().unwrap_or(1.);
                    let _ = main.set_position(tauri::PhysicalPosition::new(
                        position.x - ((preferences.width - 36.) * scale / 2.) as i32,
                        position.y,
                    ));
                }
                if let Ok(mut desktop) = handle.state::<Runtime>().lock() {
                    desktop.hwnd = main.hwnd().map(|hwnd| hwnd.0 as isize).unwrap_or(0);
                }
                let closing = handle.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if !EXITING.load(Ordering::SeqCst) {
                            api.prevent_close();
                            let _ = closing.emit_to("main", "catch:request-collapse", ());
                        }
                    }
                });
                let timeout = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(15));
                    let expired = timeout
                        .state::<Runtime>()
                        .lock()
                        .is_ok_and(|mut desktop| desktop.view.failed_open(generation));
                    if expired {
                        if let Some(main) = timeout.get_webview_window("main") {
                            let _ = main.destroy();
                        }
                        let _ = native::show(&timeout, None);
                        report(&timeout, "界面打开超时，草稿已保留；点击植物可重试".into());
                    }
                });
            }
            Err(error) => {
                if let Ok(mut desktop) = handle.state::<Runtime>().lock() {
                    desktop.view.failed_open(generation);
                }
                report(&handle, error.to_string());
            }
        }
    });
}
fn diagnostics(app: &AppHandle) {
    if let Ok(desktop) = app.state::<Runtime>().lock() {
        let windows:Vec<Value>=app.webview_windows().iter().map(|(label,window)|json!({"label":label,"visible":window.is_visible().ok(),"innerSize":window.inner_size().ok(),"scaleFactor":window.scale_factor().ok()})).collect();
        let data = json!({"pid":std::process::id(),"generation":desktop.view.generation(),"phase":desktop.view.phase(),"webviews":windows,"native":native::position(app),"time":clock::now()});
        let _ = desktop.disk.write_atomic(
            "window-diagnostics.json",
            &serde_json::to_vec_pretty(&data).unwrap_or_default(),
        );
    }
}
fn main() {
    let app=tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,_,_|expand(app)))
        .invoke_handler(tauri::generate_handler![boot,execute,stash_ui,view_painted,collapse,save_preferences,pick_resources,classify_resource,open_resource,quit])
        .setup(|app|{
            let directory=std::env::var_os("CATCH_DATA_DIR").map(PathBuf::from).unwrap_or(app.path().app_data_dir()?);std::fs::create_dir_all(&directory)?;let directory=std::fs::canonicalize(directory)?;
            if directory.file_name().is_some_and(|name|name.to_string_lossy().eq_ignore_ascii_case("float-todo")){return Err("首次迁移请使用独立目录，暂不允许直接写入原版记录".into());}
            let lock=ProfileLock::acquire(&directory).map_err(std::io::Error::other)?;let disk:Arc<dyn Disk>=Arc::new(FileDisk::new(&directory).map_err(std::io::Error::other)?);
            let now=clock::now();let day=clock::day(now);let id=uuid::Uuid::new_v4().to_string();let service=Service::open(disk.clone(),Context{now,local_day:&day,new_id:&id}).map_err(|error|std::io::Error::other(error.message))?;
            let view=Session::open(disk.clone()).map_err(std::io::Error::other)?;let preferences=match disk.read("catch-preferences.json").map_err(std::io::Error::other)?{Some(bytes)=>serde_json::from_slice(&bytes)?,None=>Preferences::default()};
            app.manage(Mutex::new(Desktop{service,view,disk,preferences,hwnd:0,day,_lock:lock,isolated:true}));
            native::install(app.handle()).map_err(std::io::Error::other)?;expand(app.handle());
            let handle=app.handle().clone();std::thread::spawn(move||{let mut ticks=0;while !EXITING.load(Ordering::SeqCst){std::thread::sleep(Duration::from_secs(1));if EXITING.load(Ordering::SeqCst){break;}ticks+=1;let now=clock::now();let day=clock::day(now);
                let result=(||->Result<Value,String>{let state=handle.state::<Runtime>();let mut desktop=state.lock().map_err(|_|"状态锁失败")?;
                    if desktop.day!=day{desktop.service.refresh_recurrences(&day).map_err(|error|error.message)?;desktop.day=day.clone();}
                    if ticks%30==0{desktop.service.checkpoint(now).map_err(|error|error.message)?;}
                    let(start,end)=clock::bounds(&day)?;let celebrate=desktop.service.observe_growth(DayBounds{day:&day,start,end},now).map_err(|error|error.message)?;
                    Ok(json!({"now":now,"growth":growth_view(&desktop,now)?,"celebrate":celebrate}))})();
                match result{Ok(tick)=>{let _=handle.emit_to("main","catch:tick",tick);},Err(error)=>report(&handle,error)}
                if ticks%30==0{emit_state(&handle);}native::redraw(&handle);
            }});Ok(())
        }).build(tauri::generate_context!()).expect("Catch startup failed");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !EXITING.load(Ordering::SeqCst) {
                api.prevent_exit();
                let _ = request_exit(app, true);
            }
        }
    });
}
