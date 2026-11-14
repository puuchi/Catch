param([ValidateSet('check','test','fmt','clippy','benchmark')][string]$Action='test')
$ErrorActionPreference='Stop'
$catchRoot=Split-Path $PSScriptRoot -Parent
# Prefer an existing user toolchain. A previous local feasibility run installed
# Rust here without changing the machine's global PATH.
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    $catchCargo=Join-Path $catchRoot 'artifacts\toolchains\cargo'
    if (!(Test-Path -LiteralPath (Join-Path $catchCargo 'bin\cargo.exe'))) {
        throw 'Install Rust from https://rustup.rs/ before running this command.'
    }
    $env:CARGO_HOME=$catchCargo
    $env:RUSTUP_HOME=Join-Path $catchRoot 'artifacts\toolchains\rustup'
    $env:PATH=(Join-Path $catchCargo 'bin')+';'+$env:PATH
}
Push-Location -LiteralPath $catchRoot
try {
    switch($Action) {
        'fmt' { & cargo fmt --all }
        'clippy' { & cargo clippy --workspace --all-targets -- -D warnings }
        'benchmark' { & cargo test -p catch-app --locked --release synthetic_history_performance_sample -- --ignored --nocapture }
        default { & cargo $Action --workspace --locked }
    }
    if($LASTEXITCODE -ne 0){throw "Rust $Action failed: $LASTEXITCODE"}
} finally { Pop-Location }
