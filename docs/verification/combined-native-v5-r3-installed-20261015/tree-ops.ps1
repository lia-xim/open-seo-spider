#requires -Version 7
param([Parameter(Mandatory)][ValidateSet('snapshot','copy','replace','cold','owner')][string]$Mode,
      [string]$Root, [string]$Destination, [string]$Stage, [string]$Out,
      [int]$ProcessId, [int]$Port)
$ErrorActionPreference='Stop'
function Ordinary([string]$path){
  $path=[IO.Path]::GetFullPath($path)
  for($p=$path;$p;$p=[IO.Path]::GetDirectoryName($p)){
    $i=Get-Item -LiteralPath $p -Force
    if($i.Attributes-band[IO.FileAttributes]::ReparsePoint){throw "Reparse path: $p"}
    if($p-eq[IO.Path]::GetPathRoot($p)){break}
  }
}
function Hash([string]$path){(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Tree([string]$root){
  Ordinary $root
  $stack=[Collections.Generic.Stack[string]]::new();$stack.Push($root)
  $rows=[Collections.Generic.List[object]]::new()
  while($stack.Count){
    $p=$stack.Pop();$i=Get-Item -LiteralPath $p -Force
    if($i.Attributes-band[IO.FileAttributes]::ReparsePoint){throw "Reparse entry: $p"}
    $acl=Get-Acl -LiteralPath $p
    $r=[ordered]@{path=[IO.Path]::GetRelativePath($root,$p);kind=$(if($i.PSIsContainer){'directory'}else{'file'});attributes=[string]$i.Attributes;created=$i.CreationTimeUtc.ToString('o');modified=$i.LastWriteTimeUtc.ToString('o');sddl=$acl.Sddl}
    if($i.PSIsContainer){foreach($c in Get-ChildItem -LiteralPath $p -Force){$stack.Push($c.FullName)}}else{
      $r.bytes=$i.Length;$r.sha256=Hash $p;$streams=@()
      foreach($s in Get-Item -LiteralPath $p -Stream '*'){
        if($s.Stream-in@(':$DATA','$DATA')){continue}
        $f=[IO.File]::OpenRead(('{0}:{1}'-f$p,$s.Stream));try{$h=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($f)).ToLowerInvariant()}finally{$f.Dispose()}
        $streams+=@{name=$s.Stream;bytes=$s.Length;sha256=$h}
      };$r.streams=@($streams|Sort-Object name)
    };$rows.Add([pscustomobject]$r)
  };return @($rows|Sort-Object path)
}
function Save($value){
  $json=ConvertTo-Json -InputObject $value -Depth 100
  if($Out){if(Test-Path -LiteralPath $Out){throw 'Output exists'};[IO.File]::WriteAllText($Out,$json,[Text.UTF8Encoding]::new($false));Write-Output('{"written":true}')}
  else{Write-Output $json}
}
if($Mode-eq'cold'){
  Ordinary $Root
  $rows=@(foreach($portNumber in @(8787,8790)){Get-NetTCPConnection -LocalPort $portNumber -State Listen -ErrorAction SilentlyContinue|Select-Object LocalAddress,LocalPort,OwningProcess})
  $exe=[IO.Path]::GetFullPath((Join-Path $Root 'target/release/open-seo-spider.exe'))
  $renderer=[IO.Path]::GetFullPath((Join-Path $Root 'renderer/server.mjs'))
  $processes=@(Get-CimInstance Win32_Process|Where-Object{$_.ExecutablePath-eq$exe-or($_.Name-eq'node.exe'-and$_.CommandLine-and$_.CommandLine.Replace('/',[string][char]92).ToLowerInvariant().Contains($renderer.ToLowerInvariant()))}|Select-Object ProcessId,ExecutablePath,CommandLine,CreationDate)
  Save @{listeners=$rows;processes=$processes};exit
}
if($Mode-eq'owner'){
  $p=Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId"
  if(-not$p){throw 'Owned process missing'}
  $listeners=@(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
  Save @{pid=$p.ProcessId;executable=$p.ExecutablePath;command_line=$p.CommandLine;created_utc=$p.CreationDate.ToUniversalTime().ToString('o');listeners=@($listeners|Select-Object LocalAddress,LocalPort,OwningProcess)};exit
}
if($Mode-eq'replace'){
  Ordinary $Root;Ordinary $Stage;Ordinary ([IO.Path]::GetDirectoryName($Destination))
  if(Test-Path -LiteralPath $Destination){throw 'Replacement backup already exists'}
  if([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Root))-ne[IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Stage))){throw 'Stage is not sibling'}
  [IO.File]::Replace($Stage,$Root,$Destination)
  Save @{replaced=$true;sha256=Hash $Root;old_backup_sha256=Hash $Destination};exit
}
$before=Tree $Root
if($Mode-eq'snapshot'){Save $before;exit}
if(Test-Path -LiteralPath $Destination){throw 'Copy destination exists'}
Ordinary ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Destination)))
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ColdDataCopyDirectory {
  [DllImport("kernel32.dll", EntryPoint="CreateDirectoryW", CharSet=CharSet.Unicode, SetLastError=true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool Create(string path, IntPtr attributes);
}
'@
if(-not[ColdDataCopyDirectory]::Create([IO.Path]::GetFullPath($Destination),[IntPtr]::Zero)){throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())}
foreach($r in $before|Where-Object kind -eq 'directory'){
  [IO.Directory]::CreateDirectory((Join-Path $Destination $r.path))|Out-Null
}
foreach($r in $before|Where-Object kind -eq 'file'){
  $src=Join-Path $Root $r.path;$dst=Join-Path $Destination $r.path
  [IO.File]::Copy($src,$dst,$false)
}
# Apply all metadata after contents, children before parents. File.Copy also copies NTFS streams;
# every stream and metadata field is then independently read back and compared.
foreach($r in $before|Sort-Object {$_.path.Length} -Descending){
  $src=Join-Path $Root $r.path;$dst=Join-Path $Destination $r.path
  Set-Acl -LiteralPath $dst -AclObject (Get-Acl -LiteralPath $src)
  $i=Get-Item -LiteralPath $dst -Force
  $i.CreationTimeUtc=[DateTime]::Parse($r.created).ToUniversalTime()
  $i.LastWriteTimeUtc=[DateTime]::Parse($r.modified).ToUniversalTime()
  $i.Attributes=[Enum]::Parse([IO.FileAttributes],$r.attributes)
}
$after=Tree $Root;$copy=Tree $Destination
$canonical={param($v) ConvertTo-Json -InputObject $v -Depth 100 -Compress}
if((&$canonical $before)-ne(&$canonical $after)){throw 'Source changed during copy'}
if((&$canonical $before)-ne(&$canonical $copy)){throw 'Backup content/metadata differs'}
Save @{verified=$true;source_before=$before;source_after=$after;copy=$copy}
