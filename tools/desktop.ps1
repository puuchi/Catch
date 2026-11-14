param(
    [ValidateSet('check','test','build','pack','run','dev','fmt','clippy')][string]$Action='check',
    [string]$TargetDirectory=''
)
$ErrorActionPreference='Stop'
$catchRoot=Split-Path $PSScriptRoot -Parent
if(!(Get-Command cargo -ErrorAction SilentlyContinue)){
    $env:CARGO_HOME=Join-Path $catchRoot 'artifacts\toolchains\cargo'
    $env:RUSTUP_HOME=Join-Path $catchRoot 'artifacts\toolchains\rustup'
    $env:PATH=(Join-Path $env:CARGO_HOME 'bin')+';'+$env:PATH
}
$env:CARGO_TARGET_DIR=if($TargetDirectory){
    $catchTargetPath=if([IO.Path]::IsPathRooted($TargetDirectory)){$TargetDirectory}else{Join-Path $catchRoot $TargetDirectory}
    [IO.Path]::GetFullPath($catchTargetPath)
}else{Join-Path $catchRoot 'artifacts\tauri-next-target'}
if(!$env:CATCH_DATA_DIR){$env:CATCH_DATA_DIR=Join-Path $catchRoot 'artifacts\catch-next-data'}
$catchCli=Join-Path $catchRoot 'frontend\node_modules\.bin\tauri.cmd'
Push-Location -LiteralPath $catchRoot
try{
    switch($Action){
        'fmt'{& cargo fmt --manifest-path src-tauri/Cargo.toml}
        'check'{& cargo check --manifest-path src-tauri/Cargo.toml}
        'test'{& cargo test --manifest-path src-tauri/Cargo.toml}
        'clippy'{& cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings}
        'build'{& $catchCli build --no-bundle}
        'pack'{& $catchCli build --no-sign}
        'dev'{& $catchCli dev}
        'run'{& (Join-Path $env:CARGO_TARGET_DIR 'release\catch-next.exe')}
    }
    if($LASTEXITCODE -ne 0){throw "Desktop $Action failed: $LASTEXITCODE"}
}finally{Pop-Location}
