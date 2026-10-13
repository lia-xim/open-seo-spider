#Requires -Version 7.2
# Tiny synthetic validator controls; NO native execution attestation.
[CmdletBinding()]
param([string]$OutputDirectory = (Join-Path ([IO.Path]::GetTempPath()) 'open-seo-spider-proof-controls'))
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$validator=Join-Path $PSScriptRoot 'verify-sitemap-transport-proof.ps1'
$runRoot=[IO.Path]::GetFullPath((Join-Path $OutputDirectory ('validator-controls-'+[Guid]::NewGuid().ToString('N'))))
$null=[IO.Directory]::CreateDirectory($runRoot)
$encoding=[Text.UTF8Encoding]::new($false)
$fakeSource='a'*40
$matrixSha='ed90247db514c4d0b1f73ebf373a1823990c1b289687a415e058c2260e716e85'
function Sha-Bytes([byte[]]$Bytes){return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Bytes)).ToLowerInvariant()}
function Sha-File([string]$File){return (Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash.ToLowerInvariant()}
function Matrix-Fixture {
    $rows=[Collections.Generic.List[object]]::new()
    foreach($format in @('plain','file-gzip')){foreach($policy in @($false,$true)){foreach($resources in @($false,$true)){
        $rows.Add([pscustomobject]@{id="$format-$([int]$policy)$([int]$resources)";policy=$policy;resources=$resources;format=$format;extension=$false;target=1})
    }}}
    foreach($format in @('misleading-gz','http-gzip','corrupt-gzip')){foreach($policy in @($false,$true)){
        $rows.Add([pscustomobject]@{id="$format-$([int]$policy)";policy=$policy;resources=$false;format=$format;extension=$false;target=$(if($format -ceq 'corrupt-gzip'){0}else{1})})
    }}
    foreach($policy in @($false,$true)){$rows.Add([pscustomobject]@{id="extension-$([int]$policy)";policy=$policy;resources=$false;format='plain';extension=$true;target=1})}
    return ,$rows.ToArray()
}
function Write-Proof($Fixture){
    [IO.File]::WriteAllText($Fixture.ProofPath,(ConvertTo-Json -InputObject $Fixture.Proof -Depth 30)+"`n",$encoding)
}
function Sync-Requests($Fixture){
    $text=(ConvertTo-Json -InputObject $Fixture.Requests -Depth 15)+"`n"
    [IO.File]::WriteAllText($Fixture.RequestsPath,$text,$encoding)
    $bytes=[IO.File]::ReadAllBytes($Fixture.RequestsPath)
    $Fixture.Proof.requests_sha256=Sha-Bytes $bytes;$Fixture.Proof.requests_bytes=$bytes.LongLength
    $Fixture.Proof.drain.total_requests=$Fixture.Requests.Count;$Fixture.Proof.drain.final_total_requests=$Fixture.Requests.Count
}
function Sync-Log($Fixture,[string]$Text){
    [IO.File]::WriteAllText($Fixture.LogPath,$Text,$encoding)
    $bytes=[IO.File]::ReadAllBytes($Fixture.LogPath)
    $Fixture.Proof.app_log_sha256=Sha-Bytes $bytes;$Fixture.Proof.app_log_bytes=$bytes.LongLength
}
function New-Fixture([string]$Name){
    $directory=Join-Path $runRoot $Name;$null=[IO.Directory]::CreateDirectory($directory)
    $binary=Join-Path $directory 'validator-only-binary.bin';$harness=Join-Path $directory 'validator-only-harness.mjs'
    [IO.File]::Copy((Join-Path $PSScriptRoot 'fixtures/validator-only-binary.bin'),$binary)
    [IO.File]::Copy((Join-Path $PSScriptRoot 'fixtures/validator-only-harness.mjs'),$harness)
    $proofPath=Join-Path $directory 'proof.json';$requestsPath=$proofPath+'.requests.json';$logPath=$proofPath+'.app.log'
    $definitions=Matrix-Fixture;$requests=[Collections.Generic.List[object]]::new();$cases=[Collections.Generic.List[object]]::new()
    $ordinal=0;$caseIndex=0
    foreach($definition in $definitions){
        $caseIndex++;$mapSuffix=if($definition.format -ceq 'misleading-gz'){'.xml.gz'}else{'.xml'}
        $requestPaths=[Collections.Generic.List[string]]::new();$requestPaths.Add('/seed-'+$definition.id);$requestPaths.Add('/map-'+$definition.id+$mapSuffix)
        if($definition.target -eq 1){$requestPaths.Add('/target-'+$definition.id)}
        foreach($route in $requestPaths){$ordinal++;$requests.Add([pscustomobject]@{ordinal=$ordinal;method='GET';path=$route;
            started_ms=$ordinal*10;finished_ms=$ordinal*10+1;response_state='finished';registered_route=$true})}
        $counts=[ordered]@{seed_requests=1;sitemap_requests=1;target_requests=$definition.target;target_pages=$definition.target;foreign_requests=0;foreign_pages=0}
        $cases.Add([pscustomobject]@{id=$definition.id;policy=$definition.policy;resources=$definition.resources;format=$definition.format;extension=$definition.extension;
            job_id=('00000000-0000-4000-8000-{0:d12}' -f $caseIndex);generation=1;terminal_status='completed';
            report_sha256=('3'*64);config_sha256=('4'*64);expected=[pscustomobject]$counts;actual=[pscustomobject]$counts;
            result='EXPECTED_BEHAVIOR_PASS';http_counts_phase='after_owned_app_close_and_clean_fixture_drain'})
    }
    $bindLine='Open SEO Spider: http://127.0.0.1:51235'
    $proof=[ordered]@{schema_version=1;result='PASS';scope_name='native_sitemap_transport_16_drained_v1';
        validator_fixture_only=$true;fixture_label='SYNTHETIC VALIDATOR DATA; NO APP/SERVER WAS LAUNCHED';
        source_commit=$fakeSource;expected_source_commit=$fakeSource;source_runtime_attested=$false;
        source_basis='caller_declared; CI checkout/build/manifest must independently bind commit to executable';
        binary_sha256=(Sha-File $binary);final_binary_sha256=(Sha-File $binary);expected_binary_sha256=(Sha-File $binary);expected_binary_sha_checked=$true;
        binary_bytes=(Get-Item -LiteralPath $binary).Length;harness_sha256=(Sha-File $harness);matrix_sha256=$matrixSha;
        final_target_http_witness='DRAINED';witness_complete=$true;owned_app_closed=$true;
        user_runtime_contacted=$false;renderer_contacted=$false;foreign_listener_or_process_controls_used=$false;
        listen_argument='127.0.0.1:0';bind_origin_basis='complete post-bind owned child stdout only';
        fixture_origin='http://127.0.0.1:51234';owned_native_origin='http://127.0.0.1:51235';owned_native_pid=314;
        bind_line_sha256=(Sha-Bytes ($encoding.GetBytes($bindLine)));owned_app_stop_requested_ms=1800;
        owned_app_close=[ordered]@{code=$null;signal='SIGTERM';closed_ms=1810};
        cleanup=[ordered]@{owned_data_removed=$true;owned_fixture_closed=$true;owned_app_closed=$true};
        drain=[ordered]@{owned_app_closed=$true;fixture_kept_listening_until_requests_drained=$true;fixture_closed_cleanly=$true;
            forced_socket_cleanup=$false;active_requests=0;connected_sockets=0;quiet_ms=250;total_requests=46;final_total_requests=46};
        summary=[ordered]@{cases=16;passes=16;failures=0;unproven=0};failures=@();cases=$cases.ToArray();
        requests_file=$requestsPath;app_log_file=$logPath;requests_sha256='';app_log_sha256='';requests_bytes=0;app_log_bytes=0}
    # Convert nested ordered dictionaries to the same object shapes as parsed real JSON.
    $parsed=ConvertFrom-Json -InputObject (ConvertTo-Json -InputObject $proof -Depth 30) -Depth 30
    $fixture=[pscustomobject]@{Directory=$directory;BinaryPath=$binary;HarnessPath=$harness;ProofPath=$proofPath;
        RequestsPath=$requestsPath;LogPath=$logPath;Proof=$parsed;Requests=$requests.ToArray()}
    Sync-Requests $fixture;Sync-Log $fixture ("STDOUT TAIL`n"+$bindLine+"`n`nSTDERR TAIL`n`n");Write-Proof $fixture
    return $fixture
}
function Invoke-Validator($Fixture){
    return & $validator -ProofPath $Fixture.ProofPath -BinaryPath $Fixture.BinaryPath -HarnessPath $Fixture.HarnessPath -SourceCommit $fakeSource
}
function Add-Late-Request($Fixture,[string]$Route){
    $Fixture.Requests=@($Fixture.Requests)+@([pscustomobject]@{ordinal=$Fixture.Requests.Count+1;method='GET';path=$Route;
        started_ms=600;finished_ms=601;response_state='finished';registered_route=$true})
    Sync-Requests $Fixture # Keep SHA/byte/total claims internally consistent; unchanged case counts must still reject.
}

$results=[Collections.Generic.List[object]]::new()
$baseline=New-Fixture 'baseline-validator-only'
$accepted=Invoke-Validator $baseline
if($accepted.result -cne 'PASS_PROOF_VALIDATION' -or $accepted.native_executable_launched_by_validator -ne $false){throw 'Synthetic baseline failed validation-only boundary'}
$results.Add([pscustomobject]@{id='baseline-validator-only';result='PASS_VALIDATOR_ONLY';native_attestation=$false})
$controls=@(
    @{id='wrong-source';code='source.commit';change={param($fixture)$fixture.Proof.source_commit='b'*40}},
    @{id='wrong-harness';code='harness.sha';change={param($fixture)$fixture.Proof.harness_sha256='0'*64}},
    @{id='stale-initial-binary';code='binary.initial_sha';change={param($fixture)$fixture.Proof.binary_sha256='0'*64}},
    @{id='stale-final-binary';code='binary.final_sha';change={param($fixture)$fixture.Proof.final_binary_sha256='0'*64}},
    @{id='undrained';code='witness.phase';change={param($fixture)$fixture.Proof.final_target_http_witness='NOT_PROVEN'}},
    @{id='truthy-string';code='witness.complete';change={param($fixture)$fixture.Proof.witness_complete='true'}},
    @{id='forced-drain';code='drain.forced';change={param($fixture)$fixture.Proof.drain.forced_socket_cleanup=$true}},
    @{id='incomplete-cleanup';code='cleanup.owned_data_removed';change={param($fixture)$fixture.Proof.cleanup.owned_data_removed=$false}},
    @{id='summary-lies';code='summary.passes';change={param($fixture)$fixture.Proof.summary.passes=15}},
    @{id='wrong-case-id';code='case.id_order';change={param($fixture)$fixture.Proof.cases[1].id=$fixture.Proof.cases[0].id}},
    @{id='case-early-phase';code='case.phase';change={param($fixture)$fixture.Proof.cases[0].http_counts_phase='before_shutdown'}},
    @{id='late-foreign-rehash';code='case.foreign_requests.witness';change={param($fixture)Add-Late-Request $fixture '/foreign-extension-0'}},
    @{id='late-corrupt-target-rehash';code='case.target_requests.witness';change={param($fixture)Add-Late-Request $fixture '/target-corrupt-gzip-0'}},
    @{id='unknown-request-rehash';code='requests.unexpected_path';change={param($fixture)Add-Late-Request $fixture '/outside-matrix'}},
    @{id='requests-companion-tamper';code='requests.sha';change={param($fixture)[IO.File]::AppendAllText($fixture.RequestsPath,' ',$encoding)}},
    @{id='log-companion-tamper';code='log.sha';change={param($fixture)[IO.File]::AppendAllText($fixture.LogPath,' ',$encoding)}},
    @{id='requests-byte-claim';code='requests.claimed_bytes';change={param($fixture)$fixture.Proof.requests_bytes++}},
    @{id='log-byte-claim';code='log.claimed_bytes';change={param($fixture)$fixture.Proof.app_log_bytes++}},
    @{id='requests-path-redirect';code='requests.reference_path';change={param($fixture)
        $trap=Join-Path $fixture.Directory 'never-open-this-path.json';[IO.File]::WriteAllText($trap,'UNTRUSTED PATH TRAP; NOT JSON',$encoding);$fixture.Proof.requests_file=$trap}},
    @{id='log-path-redirect';code='log.reference_path';change={param($fixture)
        $trap=Join-Path $fixture.Directory 'never-open-this-log.txt';[IO.File]::WriteAllText($trap,'UNTRUSTED PATH TRAP',$encoding);$fixture.Proof.app_log_file=$trap}},
    @{id='aborted-row-rehash';code='requests.response_state';change={param($fixture)$fixture.Requests[0].response_state='aborted';Sync-Requests $fixture}},
    @{id='user-port';code='ownership.native_origin';change={param($fixture)$fixture.Proof.owned_native_origin='http://127.0.0.1:8787'}},
    @{id='bind-only-in-stderr';code='log.bind_lines';change={param($fixture)Sync-Log $fixture "STDOUT TAIL`n`nSTDERR TAIL`nOpen SEO Spider: http://127.0.0.1:51235`n"}},
    @{id='invalid-utf8-log-rehash';code='log.utf8';change={param($fixture)
        [IO.File]::WriteAllBytes($fixture.LogPath,[byte[]]@(255));$fixture.Proof.app_log_sha256=Sha-File $fixture.LogPath;$fixture.Proof.app_log_bytes=1}}
)
foreach($control in $controls){
    $fixture=New-Fixture $control.id;& $control.change $fixture;Write-Proof $fixture
    $rejected=$false;$message=$null
    try{$null=Invoke-Validator $fixture}catch{$message=$_.Exception.Message;$rejected=$message.Contains('sitemap_transport_proof:'+$control.code,[StringComparison]::Ordinal)}
    if(-not $rejected){throw "Expected independent rejection $($control.id) at $($control.code); actual: $message"}
    $results.Add([pscustomobject]@{id=$control.id;result='EXPECTED_VALIDATOR_REJECTION';code=$control.code;native_attestation=$false})
}
$report=[ordered]@{schema_version=1;result='PASS_VALIDATOR_ONLY_CONTROLS';recorded_at_utc=[DateTime]::UtcNow.ToString('o');
    validator_sha256=(Sha-File $validator);controls_sha256=(Sha-File $PSCommandPath);synthetic_positive=1;negative_controls=$controls.Count;
    native_executable_launched=$false;app_or_http_server_launched=$false;website_requests=0;
    evidence_kind='Tiny synthetic stored-file fixtures; proves validator rejections only, not native execution or release correctness';
    run_directory=$runRoot;results=$results.ToArray()}
$reportPath=Join-Path $runRoot 'validator-only-controls.json'
[IO.File]::WriteAllText($reportPath,(ConvertTo-Json -InputObject $report -Depth 20)+"`n",$encoding)
[pscustomobject]@{result=$report.result;negative_controls=$controls.Count;synthetic_positive=1;native_execution=$false;report=$reportPath}
