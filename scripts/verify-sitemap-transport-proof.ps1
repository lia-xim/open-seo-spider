#Requires -Version 7.2
# Validates stored evidence; never launches a binary/server or copies files.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ProofPath,
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][string]$HarnessPath,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceCommit
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Require-Condition {
    param([bool]$Condition,[string]$Code)
    if(-not $Condition){throw "sitemap_transport_proof:$Code"}
}
function Field {
    param([object]$Document,[string]$Name)
    Require-Condition ($null -ne $Document -and $Document -is [pscustomobject]) "object.$Name"
    $property=$Document.PSObject.Properties[$Name]
    Require-Condition ($null -ne $property) "missing.$Name"
    return ,$property.Value
}
function Has-Field {
    param([object]$Document,[string]$Name)
    return $null -ne $Document -and $null -ne $Document.PSObject.Properties[$Name]
}
function Require-Text {
    param([object]$Value,[string]$Expected,[string]$Code)
    Require-Condition ($Value -is [string] -and [string]::Equals($Value,$Expected,[StringComparison]::Ordinal)) $Code
}
function Require-Flag {
    param([object]$Value,[bool]$Expected,[string]$Code)
    Require-Condition ($Value -is [bool] -and $Value -eq $Expected) $Code
}
function Require-Integer {
    param([object]$Value,[long]$Minimum,[long]$Maximum,[string]$Code)
    $integer=$Value -is [byte] -or $Value -is [sbyte] -or $Value -is [int16] -or $Value -is [uint16] -or
        $Value -is [int32] -or $Value -is [uint32] -or $Value -is [int64] -or $Value -is [uint64]
    Require-Condition ($integer -and $Value -ge $Minimum -and $Value -le $Maximum) $Code
}
function Require-Count {
    param([object]$Value,[long]$Expected,[string]$Code)
    Require-Integer $Value 0 9007199254740991 $Code
    Require-Condition ($Value -eq $Expected) $Code
}
function Require-Sha {
    param([object]$Value,[string]$Expected,[string]$Code)
    Require-Condition ($Value -is [string] -and $Value -cmatch '^[0-9a-f]{64}$') $Code
    Require-Text $Value $Expected $Code
}
function Normalize-Path {
    param([string]$Value)
    return [System.IO.Path]::GetFullPath($Value)
}
function Same-Path {
    param([string]$Left,[string]$Right)
    $comparison=if([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)){
        [StringComparison]::OrdinalIgnoreCase
    }else{[StringComparison]::Ordinal}
    return [string]::Equals((Normalize-Path $Left),(Normalize-Path $Right),$comparison)
}
function Trusted-File {
    param([string]$Value,[long]$MaximumBytes,[string]$Code)
    $resolved=Normalize-Path $Value
    $item=Get-Item -LiteralPath $resolved -Force
    Require-Condition ($item -is [System.IO.FileInfo] -and -not $item.PSIsContainer) "$Code.file"
    Require-Condition (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) "$Code.link"
    Require-Condition ($item.Length -gt 0 -and $item.Length -le $MaximumBytes) "$Code.bytes"
    return $item
}
function Bytes-Sha {
    param([byte[]]$Bytes)
    return [Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($Bytes)).ToLowerInvariant()
}
function Full-Bytes {
    param([System.IO.FileInfo]$File,[string]$Code)
    $bytes=[System.IO.File]::ReadAllBytes($File.FullName)
    Require-Condition ($bytes.LongLength -eq $File.Length) "$Code.changed_bytes"
    return ,$bytes
}
function Strict-Utf8 {
    param([byte[]]$Bytes,[string]$Code)
    try{return [System.Text.UTF8Encoding]::new($false,$true).GetString($Bytes)}
    catch{throw "sitemap_transport_proof:$Code.utf8"}
}
function Parse-Json {
    param([byte[]]$Bytes,[string]$Code)
    $text=Strict-Utf8 $Bytes $Code
    try{return ,(ConvertFrom-Json -InputObject $text -Depth 100 -NoEnumerate)}
    catch{throw "sitemap_transport_proof:$Code.json"}
}
function Require-Reference {
    param([object]$Value,[string]$TrustedPath,[string]$Code)
    Require-Condition ($Value -is [string] -and [System.IO.Path]::IsPathRooted($Value)) $Code
    # Comparison only. NEVER read a path supplied by JSON.
    try{$equal=Same-Path $Value $TrustedPath}catch{throw "sitemap_transport_proof:$Code"}
    Require-Condition $equal $Code
}
function Loopback-Origin {
    param([object]$Value,[string]$Code)
    Require-Condition ($Value -is [string]) $Code
    $parsed=$null
    Require-Condition ([Uri]::TryCreate($Value,[UriKind]::Absolute,[ref]$parsed)) $Code
    Require-Condition ($parsed.Scheme -ceq 'http' -and $parsed.Host -ceq '127.0.0.1' -and
        $parsed.Port -gt 0 -and $parsed.Port -le 65535 -and $parsed.Port -notin @(8787,8790) -and
        $parsed.AbsolutePath -ceq '/' -and $parsed.Query -ceq '' -and $parsed.Fragment -ceq '' -and $parsed.UserInfo -ceq '') $Code
}
function Matrix {
    $result=[System.Collections.Generic.List[object]]::new()
    foreach($policy in @($false,$true)){foreach($resources in @($false,$true)){
        $suffix=([int]$policy).ToString()+([int]$resources).ToString()
        $result.Add([pscustomobject]@{id="plain-$suffix";policy=$policy;resources=$resources;format='plain';extension=$false;target=1})
    }}
    foreach($policy in @($false,$true)){foreach($resources in @($false,$true)){
        $suffix=([int]$policy).ToString()+([int]$resources).ToString()
        $result.Add([pscustomobject]@{id="file-gzip-$suffix";policy=$policy;resources=$resources;format='file-gzip';extension=$false;target=1})
    }}
    foreach($format in @('misleading-gz','http-gzip','corrupt-gzip')){foreach($policy in @($false,$true)){
        $target=if($format -ceq 'corrupt-gzip'){0}else{1}
        $result.Add([pscustomobject]@{id="$format-$([int]$policy)";policy=$policy;resources=$false;format=$format;extension=$false;target=$target})
    }}
    foreach($policy in @($false,$true)){
        $result.Add([pscustomobject]@{id="extension-$([int]$policy)";policy=$policy;resources=$false;format='plain';extension=$true;target=1})
    }
    return ,$result.ToArray()
}

# All opened files derive from trusted parameters. Their JSON path fields are comparisons only.
$trustedProof=Normalize-Path $ProofPath
$trustedRequests=$trustedProof+'.requests.json'
$trustedLog=$trustedProof+'.app.log'
$proofFile=Trusted-File $trustedProof (16*1024*1024) 'proof'
$binaryFile=Trusted-File $BinaryPath ([long]2147483647) 'binary'
$harnessFile=Trusted-File $HarnessPath (4*1024*1024) 'harness'
$proofBytes=Full-Bytes $proofFile 'proof'
$proof=Parse-Json $proofBytes 'proof'
$binarySha=(Get-FileHash -LiteralPath $binaryFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$harnessSha=(Get-FileHash -LiteralPath $harnessFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
Require-Count (Field $proof 'schema_version') 1 'proof.schema'
Require-Text (Field $proof 'result') 'PASS' 'proof.result'
Require-Text (Field $proof 'scope_name') 'native_sitemap_transport_16_drained_v1' 'proof.scope'
Require-Sha (Field $proof 'binary_sha256') $binarySha 'binary.initial_sha'
Require-Sha (Field $proof 'final_binary_sha256') $binarySha 'binary.final_sha'
Require-Count (Field $proof 'binary_bytes') $binaryFile.Length 'binary.bytes'
Require-Sha (Field $proof 'harness_sha256') $harnessSha 'harness.sha'
Require-Text (Field $proof 'source_commit') $SourceCommit 'source.commit'
$expectedSource=Field $proof 'expected_source_commit'
if($null -ne $expectedSource){Require-Text $expectedSource $SourceCommit 'source.expected_commit'}
$expectedBinary=Field $proof 'expected_binary_sha256'
if($null -ne $expectedBinary){
    Require-Sha $expectedBinary $binarySha 'binary.expected_sha'
    Require-Flag (Field $proof 'expected_binary_sha_checked') $true 'binary.expected_checked'
}else{Require-Flag (Field $proof 'expected_binary_sha_checked') $false 'binary.expected_checked'}
Require-Flag (Field $proof 'source_runtime_attested') $false 'source.runtime_attestation'
Require-Text (Field $proof 'source_basis') 'caller_declared; CI checkout/build/manifest must independently bind commit to executable' 'source.basis'
Require-Sha (Field $proof 'matrix_sha256') 'ed90247db514c4d0b1f73ebf373a1823990c1b289687a415e058c2260e716e85' 'matrix.sha'
Require-Text (Field $proof 'final_target_http_witness') 'DRAINED' 'witness.phase'
Require-Flag (Field $proof 'witness_complete') $true 'witness.complete'
Require-Flag (Field $proof 'owned_app_closed') $true 'cleanup.app_closed'
Require-Flag (Field $proof 'user_runtime_contacted') $false 'ownership.user_runtime'
Require-Flag (Field $proof 'renderer_contacted') $false 'ownership.renderer'
Require-Flag (Field $proof 'foreign_listener_or_process_controls_used') $false 'ownership.foreign_controls'
Require-Text (Field $proof 'listen_argument') '127.0.0.1:0' 'ownership.listen_argument'
Require-Text (Field $proof 'bind_origin_basis') 'complete post-bind owned child stdout only' 'ownership.bind_basis'
$fixtureOrigin=Field $proof 'fixture_origin';$nativeOrigin=Field $proof 'owned_native_origin'
Loopback-Origin $fixtureOrigin 'ownership.fixture_origin';Loopback-Origin $nativeOrigin 'ownership.native_origin'
Require-Condition (-not [string]::Equals($fixtureOrigin,$nativeOrigin,[StringComparison]::Ordinal)) 'ownership.distinct_origins'
Require-Integer (Field $proof 'owned_native_pid') 1 2147483647 'ownership.pid'
$stopAt=Field $proof 'owned_app_stop_requested_ms'
Require-Integer $stopAt 0 9007199254740991 'ownership.stop_time'
$closed=Field $proof 'owned_app_close'
$closeAt=Field $closed 'closed_ms'
Require-Integer $closeAt 0 9007199254740991 'ownership.close_time'
Require-Condition ($closeAt -ge $stopAt) 'ownership.close_before_stop'
$cleanup=Field $proof 'cleanup'
foreach($flag in @('owned_data_removed','owned_fixture_closed','owned_app_closed')){
    Require-Flag (Field $cleanup $flag) $true "cleanup.$flag"
}
$drain=Field $proof 'drain'
Require-Flag (Field $drain 'owned_app_closed') $true 'drain.app_closed'
Require-Flag (Field $drain 'fixture_kept_listening_until_requests_drained') $true 'drain.listener'
Require-Flag (Field $drain 'fixture_closed_cleanly') $true 'drain.clean'
Require-Flag (Field $drain 'forced_socket_cleanup') $false 'drain.forced'
Require-Count (Field $drain 'active_requests') 0 'drain.active'
Require-Count (Field $drain 'connected_sockets') 0 'drain.sockets'
Require-Integer (Field $drain 'quiet_ms') 250 8000 'drain.quiet'
$summary=Field $proof 'summary'
Require-Count (Field $summary 'cases') 16 'summary.cases'
Require-Count (Field $summary 'passes') 16 'summary.passes'
Require-Count (Field $summary 'failures') 0 'summary.failures'
Require-Count (Field $summary 'unproven') 0 'summary.unproven'
$failures=Field $proof 'failures'
Require-Condition ($failures -is [array] -and $failures.Count -eq 0) 'proof.failures'
Require-Reference (Field $proof 'requests_file') $trustedRequests 'requests.reference_path'
Require-Reference (Field $proof 'app_log_file') $trustedLog 'log.reference_path'

$requestsFile=Trusted-File $trustedRequests (16*1024*1024) 'requests'
$logFile=Trusted-File $trustedLog (256*1024) 'log'
$requestBytes=Full-Bytes $requestsFile 'requests';$logBytes=Full-Bytes $logFile 'log'
$requestsSha=Bytes-Sha $requestBytes;$logSha=Bytes-Sha $logBytes
Require-Sha (Field $proof 'requests_sha256') $requestsSha 'requests.sha'
Require-Sha (Field $proof 'app_log_sha256') $logSha 'log.sha'
if(Has-Field $proof 'requests_bytes'){Require-Count (Field $proof 'requests_bytes') $requestBytes.LongLength 'requests.claimed_bytes'}
if(Has-Field $proof 'app_log_bytes'){Require-Count (Field $proof 'app_log_bytes') $logBytes.LongLength 'log.claimed_bytes'}
$requests=Parse-Json $requestBytes 'requests';$log=Strict-Utf8 $logBytes 'log'
Require-Condition ($requests -is [array] -and $requests.Count -gt 0 -and $requests.Count -le 1024) 'requests.rows'
Require-Count (Field $drain 'total_requests') $requests.Count 'drain.total_requests'
Require-Count (Field $drain 'final_total_requests') $requests.Count 'drain.final_total_requests'

# Check the recorded bind line only in the bounded stdout part, never stderr.
$stdoutPrefix="STDOUT TAIL`n";$stderrMarker="`nSTDERR TAIL`n"
Require-Condition ($log.StartsWith($stdoutPrefix,[StringComparison]::Ordinal)) 'log.stdout_frame'
$stderrOffset=$log.IndexOf($stderrMarker,$stdoutPrefix.Length,[StringComparison]::Ordinal)
Require-Condition ($stderrOffset -ge 0) 'log.stderr_frame'
$stdout=$log.Substring($stdoutPrefix.Length,$stderrOffset-$stdoutPrefix.Length)
$bindLines=[regex]::Matches($stdout,'(?m)^Open SEO Spider: http://127\.0\.0\.1:[1-9][0-9]*\r?$')
Require-Condition ($bindLines.Count -eq 1) 'log.bind_lines'
$bindLine=$bindLines[0].Value
Require-Text ($bindLine.TrimEnd([char]13)) ('Open SEO Spider: '+$nativeOrigin) 'log.bind_origin'
Require-Sha (Field $proof 'bind_line_sha256') (Bytes-Sha ([System.Text.Encoding]::UTF8.GetBytes($bindLine))) 'log.bind_sha'

$pathCounts=[System.Collections.Generic.Dictionary[string,long]]::new([StringComparer]::Ordinal)
$ordinal=0
foreach($request in $requests){
    $ordinal++
    Require-Count (Field $request 'ordinal') $ordinal 'requests.ordinal'
    Require-Text (Field $request 'method') 'GET' 'requests.method'
    Require-Text (Field $request 'response_state') 'finished' 'requests.response_state'
    Require-Flag (Field $request 'registered_route') $true 'requests.registered_route'
    if(Has-Field $request 'request_error'){Require-Condition ($null -eq (Field $request 'request_error')) 'requests.request_error'}
    $start=Field $request 'started_ms';$finish=Field $request 'finished_ms'
    Require-Integer $start 0 9007199254740991 'requests.start_time'
    Require-Integer $finish 0 9007199254740991 'requests.finish_time'
    Require-Condition ($finish -ge $start) 'requests.finish_before_start'
    $requestPath=Field $request 'path'
    Require-Condition ($requestPath -is [string] -and $requestPath.Length -le 8192 -and $requestPath.StartsWith('/')) 'requests.path'
    if($pathCounts.ContainsKey($requestPath)){$pathCounts[$requestPath]++}else{$pathCounts.Add($requestPath,1)}
}
$cases=Field $proof 'cases';$definitions=Matrix
Require-Condition ($cases -is [array] -and $cases.Count -eq 16) 'cases.count'
$allowedPaths=[System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
for($index=0;$index -lt $definitions.Count;$index++){
    $definition=$definitions[$index];$case=$cases[$index];$caseId=$definition.id
    Require-Text (Field $case 'id') $caseId 'case.id_order'
    Require-Flag (Field $case 'policy') $definition.policy 'case.policy'
    Require-Flag (Field $case 'resources') $definition.resources 'case.resources'
    Require-Flag (Field $case 'extension') $definition.extension 'case.extension'
    Require-Text (Field $case 'format') $definition.format 'case.format'
    Require-Text (Field $case 'result') 'EXPECTED_BEHAVIOR_PASS' 'case.result'
    Require-Text (Field $case 'http_counts_phase') 'after_owned_app_close_and_clean_fixture_drain' 'case.phase'
    Require-Text (Field $case 'terminal_status') 'completed' 'case.terminal'
    $jobId=Field $case 'job_id'
    Require-Condition ($jobId -is [string] -and $jobId -cmatch '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$') 'case.job_id'
    foreach($name in @('report_sha256','config_sha256')){
        $value=Field $case $name;Require-Condition ($value -is [string] -and $value -cmatch '^[0-9a-f]{64}$') "case.$name"
    }
    $expectations=[ordered]@{seed_requests=1;sitemap_requests=1;target_requests=$definition.target;
        target_pages=$definition.target;foreign_requests=0;foreign_pages=0}
    $mapSuffix=if($definition.format -ceq 'misleading-gz'){'.xml.gz'}else{'.xml'}
    $routes=[ordered]@{seed_requests="/seed-$caseId";sitemap_requests="/map-$caseId$mapSuffix";
        target_requests="/target-$caseId";foreign_requests="/foreign-$caseId"}
    $expected=Field $case 'expected';$actual=Field $case 'actual'
    foreach($name in $expectations.Keys){
        Require-Count (Field $expected $name) $expectations[$name] "case.$name.expected"
        Require-Count (Field $actual $name) $expectations[$name] "case.$name.actual"
    }
    foreach($name in $routes.Keys){
        $route=$routes[$name];$null=$allowedPaths.Add($route)
        $count=if($pathCounts.ContainsKey($route)){$pathCounts[$route]}else{0}
        Require-Count $count $expectations[$name] "case.$name.witness"
    }
}
foreach($route in $pathCounts.Keys){Require-Condition ($allowedPaths.Contains($route)) 'requests.unexpected_path'}
Require-Count $requests.Count 46 'requests.exact_matrix_total'

# Recheck actual trusted bytes after validation to detect concurrent changes; no native execution.
Require-Sha ((Get-FileHash -LiteralPath $binaryFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()) $binarySha 'binary.changed_during_validation'
Require-Sha ((Get-FileHash -LiteralPath $harnessFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()) $harnessSha 'harness.changed_during_validation'
Require-Sha ((Get-FileHash -LiteralPath $proofFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()) (Bytes-Sha $proofBytes) 'proof.changed_during_validation'
Require-Sha ((Get-FileHash -LiteralPath $requestsFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()) $requestsSha 'requests.changed_during_validation'
Require-Sha ((Get-FileHash -LiteralPath $logFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()) $logSha 'log.changed_during_validation'
[pscustomobject]@{
    result='PASS_PROOF_VALIDATION';validation_basis='stored evidence consistency and trusted-file hashes; not independent native execution attestation'
    native_executable_launched_by_validator=$false;server_launched_by_validator=$false;website_requests_by_validator=0
    source_commit=$SourceCommit;binary_sha256=$binarySha;binary_bytes=$binaryFile.Length;harness_sha256=$harnessSha
    proof_path=$trustedProof;proof_sha256=(Bytes-Sha $proofBytes);proof_bytes=$proofBytes.LongLength
    requests_path=$trustedRequests;requests_sha256=$requestsSha;requests_bytes=$requestBytes.LongLength
    app_log_path=$trustedLog;app_log_sha256=$logSha;app_log_bytes=$logBytes.LongLength
    requests_bytes_claim_checked=(Has-Field $proof 'requests_bytes');app_log_bytes_claim_checked=(Has-Field $proof 'app_log_bytes')
    cases=16;passes=16;final_target_http_witness='DRAINED';request_rows=46;source_runtime_attested=$false
}
