param(
    [Parameter(Mandatory)][int]$RootProcessId,
    [ValidateRange(1,60)][int]$SampleSeconds=3,
    [ValidateRange(1,100)][int]$Samples=3,
    [string]$DiagnosticPath='',
    [ValidateSet('unspecified','expanded','bubble')][string]$ExpectedPhase='unspecified',
    [string]$SceneLabel='unspecified',
    [string]$OutputPath=''
)
$ErrorActionPreference='Stop'
function Get-CatchTree {
    $all=@(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath)
    $ids=[Collections.Generic.HashSet[int]]::new()
    $null=$ids.Add($RootProcessId)
    do {
        $changed=$false
        foreach($item in $all){
            if($ids.Contains([int]$item.ParentProcessId) -and $ids.Add([int]$item.ProcessId)){$changed=$true}
        }
    } while($changed)
    @($all | Where-Object {$ids.Contains([int]$_.ProcessId)})
}
function Get-CatchDeclaredScene {
    if(!$DiagnosticPath){return $null}
    try {
        $data=Get-Content -LiteralPath $DiagnosticPath -Raw | ConvertFrom-Json
        $views=@($data.webviews)
        $valid=[int]$data.pid -eq $RootProcessId
        if($ExpectedPhase -eq 'expanded'){
            $valid=$valid -and $data.phase -eq 'expanded' -and @($views | Where-Object {$_.visible -eq $true}).Count -eq 1
        } elseif($ExpectedPhase -eq 'bubble') {
            $valid=$valid -and $data.phase -eq 'bubble' -and $views.Count -eq 0 -and $null -ne $data.native
        } else {$valid=$false}
        [pscustomobject]@{Pid=$data.pid;Phase=$data.phase;Generation=$data.generation;WebViews=$views.Count;ReportedAt=$data.time;MatchesExpected=$valid}
    } catch {
        [pscustomobject]@{MatchesExpected=$false;Error=$_.Exception.Message}
    }
}
$root=Get-Process -Id $RootProcessId
$rootStart=$root.StartTime.ToUniversalTime().ToString('o')
$appPath=$root.Path
$appHash=if($appPath){(Get-FileHash -LiteralPath $appPath -Algorithm SHA256).Hash}else{$null}
$system=Get-CimInstance Win32_OperatingSystem
$measurements=@()
for($sample=0;$sample -lt $Samples;$sample++){
    $sceneBefore=Get-CatchDeclaredScene
    $beforeTree=@(Get-CatchTree)
    if(!($beforeTree | Where-Object ProcessId -eq $RootProcessId)){throw 'Root process no longer exists'}
    $cpuBefore=@{}
    foreach($item in $beforeTree){
        $process=Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
        if($process){$cpuBefore[[int]$item.ProcessId]=$process.TotalProcessorTime.TotalMilliseconds}
    }
    $watch=[Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $SampleSeconds
    $afterTree=@(Get-CatchTree)
    $afterIds=@($afterTree | ForEach-Object {[int]$_.ProcessId})
    $counters=@(Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object {$_.IDProcess -in $afterIds})
    $cpu=0.0; $commit=0L; $workingSet=0L; $processesRead=0
    foreach($item in $afterTree){
        $process=Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
        if($process){
            $processesRead++
            $commit+=$process.PrivateMemorySize64; $workingSet+=$process.WorkingSet64
            if($cpuBefore.ContainsKey([int]$item.ProcessId)){$cpu += [Math]::Max(0,$process.TotalProcessorTime.TotalMilliseconds-$cpuBefore[[int]$item.ProcessId])}
        }
    }
    $watch.Stop()
    $liveRoot=Get-Process -Id $RootProcessId -ErrorAction Stop
    if($liveRoot.StartTime.ToUniversalTime().ToString('o') -ne $rootStart){throw 'Root process identity changed'}
    $treeStable=@(Compare-Object @($beforeTree.ProcessId | Sort-Object) @($afterTree.ProcessId | Sort-Object)).Count -eq 0
    $sceneAfter=Get-CatchDeclaredScene
    $sceneStable=if($null -eq $sceneBefore -or $null -eq $sceneAfter){$null}else{
        $sceneBefore.MatchesExpected -and $sceneAfter.MatchesExpected -and $sceneBefore.Generation -eq $sceneAfter.Generation
    }
    $complete=$counters.Count -eq $afterIds.Count -and $processesRead -eq $afterIds.Count
    $measurements += [pscustomobject]@{
        TimestampUtc=[DateTime]::UtcNow.ToString('o');ProcessCount=$afterIds.Count;ProcessTreeStable=$treeStable
        Names=@($afterTree.Name);CounterCoverageComplete=$complete
        PrivateWorkingSetBytes=$(if($complete){[long](($counters | Measure-Object WorkingSetPrivate -Sum).Sum)}else{$null})
        PrivateCommitBytes=$(if($complete){$commit}else{$null});SummedWorkingSetBytes=$(if($complete){$workingSet}else{$null})
        CpuPercentOneCore=[Math]::Round($cpu/$watch.Elapsed.TotalMilliseconds*100,3)
        CpuPercentAllCores=[Math]::Round($cpu/$watch.Elapsed.TotalMilliseconds*100/[Environment]::ProcessorCount,3)
        HostSceneBefore=$sceneBefore;HostSceneAfter=$sceneAfter;HostDeclaredSceneStable=$sceneStable
    }
}
$webview=$afterTree | Where-Object Name -eq 'msedgewebview2.exe' | Select-Object -First 1
$report=[pscustomobject]@{
    SceneLabel=$SceneLabel;RootProcessId=$RootProcessId;ProcessStartedUtc=$rootStart;ExecutableSha256=$appHash
    WindowsVersion=$system.Version;WindowsBuild=$system.BuildNumber
    WebView2Version=$(if($webview.ExecutablePath){[Diagnostics.FileVersionInfo]::GetVersionInfo($webview.ExecutablePath).FileVersion}else{$null})
    LogicalProcessors=[Environment]::ProcessorCount;ExpectedPhase=$ExpectedPhase;Samples=$measurements
    Limitations='Short process-tree observation. Host diagnostics are not direct visual verification. Private working set excludes shared pages; commit is not resident RAM; summed working sets double-count shared pages. CPU is comparable only for stable process trees. No diagnostic input means scene stability is unverified, never assumed true.'
}
$json=$report | ConvertTo-Json -Depth 9
if($OutputPath){$json | Set-Content -LiteralPath $OutputPath -Encoding utf8}
$json
