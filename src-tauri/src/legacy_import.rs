use crate::{ensure, growth_view, native, Context, Runtime};
use catch_storage::legacy::{self, Source};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
pub(crate) struct Pending {
    directory: PathBuf,
    source: Source,
}
fn read_source(directory: &Path) -> Result<Source, String> {
    for name in [
        "catch-import-pending.json",
        "catch-archive-pending.json",
        "catch-schedule-pending.json",
        "catch-recurrences.json",
    ] {
        if directory.join(name).exists() {
            return Err("此目录包含新版数据或未完成事务，请使用旧版已关闭后的数据目录".into());
        }
    }
    let mut files = [None, None, None];
    for (index, name) in legacy::FILES.iter().enumerate() {
        let path = directory.join(name);
        let file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if directory.join(format!("{name}.bak")).exists() {
                    return Err(format!("{name} 缺失但备份存在，请先在旧版恢复"));
                }
                continue;
            }
            Err(error) => return Err(format!("无法读取 {name}：{error}")),
        };
        let mut bytes = vec![];
        file.take(100_000_001)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() > 100_000_000 {
            return Err("单个导入文件超过 100 MB，请先备份并检查记录".into());
        }
        files[index] = Some(bytes);
    }
    Ok(Source(files))
}
#[tauri::command]
pub async fn preview_legacy(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Runtime>,
    generation: u64,
) -> Result<Option<Value>, String> {
    {
        let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
        ensure(&window, &desktop, generation)?;
        desktop.pending_import = None;
    }
    let selection = tauri::async_runtime::spawn_blocking(move || {
        let Some(directory) = rfd::FileDialog::new()
            .set_parent(&window)
            .set_title("选择已退出的旧版数据文件夹")
            .pick_folder()
        else {
            return Ok(None);
        };
        let source = read_source(&directory)?;
        let summary = legacy::inspect(&source, crate::clock::now())?;
        if read_source(&directory)? != source {
            return Err("旧版记录仍在变化，请先退出旧版再试".into());
        }
        Ok::<_, String>(Some((Pending { directory, source }, summary)))
    })
    .await
    .map_err(|error| error.to_string())??;
    let Some((pending, summary)) = selection else {
        return Ok(None);
    };
    let state = app.state::<Runtime>();
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    desktop.view.ensure(generation)?;
    let directory = pending.directory.to_string_lossy().to_string();
    desktop.pending_import = Some(pending);
    Ok(Some(json!({"directory":directory,"summary":summary})))
}
#[tauri::command]
pub fn commit_legacy(
    window: WebviewWindow,
    app: AppHandle,
    state: State<Runtime>,
    generation: u64,
) -> Result<Value, String> {
    let mut desktop = state.lock().map_err(|_| "状态锁失败")?;
    ensure(&window, &desktop, generation)?;
    let pending = desktop
        .pending_import
        .take()
        .ok_or("请先预览要导入的记录")?;
    if read_source(&pending.directory)? != pending.source {
        return Err("预览后旧版记录已变化，请退出旧版后重新选择".into());
    }
    let now = crate::clock::now();
    let day = crate::clock::day(now);
    let id = uuid::Uuid::new_v4().to_string();
    let disk = desktop.disk.clone();
    let result = desktop.service.import_legacy(
        disk,
        &pending.source,
        Context {
            now,
            local_day: &day,
            new_id: &id,
        },
    );
    let snapshot = desktop.service.snapshot();
    let growth = growth_view(&desktop, now)?;
    let _ = app.emit_to("main", "catch:state", &snapshot);
    drop(desktop);
    native::redraw(&app);
    Ok(match result {
        Ok(_) => json!({"ok":true,"state":snapshot,"growth":growth,"now":now}),
        Err(error) => json!({"ok":false,"state":snapshot,"growth":growth,"now":now,"error":error}),
    })
}
