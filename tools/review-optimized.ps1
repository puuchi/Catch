$ErrorActionPreference='Stop'
$catchProject=Split-Path $PSScriptRoot -Parent
$catchBinary=Join-Path $catchProject 'artifacts\tauri-optimized-target\release\catch-next.exe'
if(!(Test-Path -LiteralPath $catchBinary)){throw '请先运行 tools/desktop.ps1 -Action pack -TargetDirectory artifacts/tauri-optimized-target'}
if(Get-Process catch-next -ErrorAction SilentlyContinue){
    throw '请先从窗口底部退出当前的本地预览，再运行此脚本。原版 Electron Catch 可以继续运行。'
}
$catchReviewDirectory=Join-Path $catchProject 'artifacts\catch-optimized-review'
$null=New-Item -ItemType Directory -Path $catchReviewDirectory -Force
# Seed only a completely new review profile; subsequent reviews retain edits.
$catchExisting=Get-ChildItem -LiteralPath $catchReviewDirectory -Force | Select-Object -First 1
if(!$catchExisting){
    $catchExamples=@(
        @{id='optimized-review-read';text='读一篇一直想读的文章';done=$false;notes='先读一小段，记下一点收获。';urgent=$false;deadline=$null;completedAt=$null},
        @{id='optimized-review-material';text='整理项目的参考材料';done=$false;notes='可以在这里试试关联资料和设置重复。';urgent=$false;deadline=$null;completedAt=$null}
    )
    [IO.File]::WriteAllText((Join-Path $catchReviewDirectory 'todos.json'),($catchExamples | ConvertTo-Json -Depth 4),[Text.UTF8Encoding]::new($false))
}
$catchPreviousData=$env:CATCH_DATA_DIR
$catchLogSuffix=Get-Date -Format 'yyyyMMdd-HHmmss-fff'
try {
    $env:CATCH_DATA_DIR=$catchReviewDirectory
    $catchReviewProcess=Start-Process -FilePath $catchBinary -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $catchReviewDirectory "stdout-$catchLogSuffix.log") `
        -RedirectStandardError (Join-Path $catchReviewDirectory "stderr-$catchLogSuffix.log")
    [pscustomobject]@{Pid=$catchReviewProcess.Id;Executable=$catchBinary;SyntheticData=$catchReviewDirectory}
} finally {
    $env:CATCH_DATA_DIR=$catchPreviousData
}
