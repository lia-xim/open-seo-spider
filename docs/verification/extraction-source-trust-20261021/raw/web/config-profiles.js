'use strict';
// JSON text is authoritative. In particular, never round a native u64 by
// parsing and reserializing a complete configuration in JavaScript.
(function (host) {
  const KEY = 'open-seo-spider-config-profile-draft-v1';
  const ENGINE = 'open-seo-spider/crawl-config-profile-v1';
  const MAX_BYTES = 67108864;
  const size = text => new TextEncoder().encode(text).length;
  function fields(text) {
    if (typeof text !== 'string') throw Error('Ein JSON-Text ist erforderlich.');
    let i = 0;
    const ws = () => { while (/\s/.test(text[i] || '') && i < text.length) i++; };
    function stringEnd() {
      if (text[i++] !== '"') throw Error('Ungültiger JSON-Schlüssel.');
      while (i < text.length) {
        const ch = text[i++];
        if (ch === '\\') { i++; continue; }
        if (ch === '"') return;
      }
      throw Error('Unvollständiger JSON-Text.');
    }
    function valueEnd() {
      if (text[i] === '"') { stringEnd(); return; }
      if (text[i] === '{' || text[i] === '[') {
        const stack = [text[i++] === '{' ? '}' : ']'];
        while (i < text.length && stack.length) {
          const ch = text[i];
          if (ch === '"') { stringEnd(); continue; }
          i++;
          if (ch === '{' || ch === '[') stack.push(ch === '{' ? '}' : ']');
          else if (ch === '}' || ch === ']') { if (stack.pop() !== ch) throw Error('Ungültiger JSON-Text.'); }
        }
        if (stack.length) throw Error('Unvollständiger JSON-Text.');
        return;
      }
      while (i < text.length && !/[\s,}\]]/.test(text[i])) i++;
    }
    ws(); if (text[i++] !== '{') throw Error('Ein JSON-Objekt ist erforderlich.');
    const result = new Map(); ws();
    while (text[i] !== '}') {
      const keyStart = i; stringEnd(); const key = JSON.parse(text.slice(keyStart, i));
      if (result.has(key)) throw Error('Doppelter JSON-Schlüssel.');
      ws(); if (text[i++] !== ':') throw Error('Ungültiger JSON-Text.'); ws();
      const start = i; valueEnd();
      if (i === start) throw Error('Fehlender JSON-Wert.');
      result.set(key, text.slice(start, i)); ws();
      if (text[i] === '}') break;
      if (text[i++] !== ',') throw Error('Ungültiger JSON-Text.'); ws();
    }
    if (text[i++] !== '}') throw Error('Unvollständiger JSON-Text.'); ws();
    if (i !== text.length) throw Error('Zusätzliche Daten nach dem JSON-Objekt.');
    return result;
  }
  function at(text, path) {
    let value = text;
    for (const key of path) {
      if (value === undefined || value.trim() === 'null') return undefined;
      value = fields(value).get(key);
    }
    return value;
  }
  function toMiB(raw) {
    if (!/^\d+$/.test(raw)) throw Error('Ungültige native Bytezahl.');
    const bytes = BigInt(raw), divisor = 1048576n;
    const whole = bytes / divisor, remainder = bytes % divisor;
    if (!remainder) return String(whole);
    return String(whole) + '.' + String(remainder * 100000000000000000000n / divisor).padStart(20, '0').replace(/0+$/, '');
  }
  function fromMiB(raw) {
    const match = /^(\d+)(?:\.(\d+))?$/.exec(raw.trim());
    if (!match || (match[2] || '').length > 40) throw Error('Bodybudget: Eine positive exakte MiB-Zahl eingeben.');
    const scale = 10n ** BigInt((match[2] || '').length);
    const numerator = (BigInt(match[1]) * scale + BigInt(match[2] || '0')) * 1048576n;
    if (numerator % scale || numerator <= 0n) throw Error('Bodybudget: Der MiB-Wert muss eine ganze positive Bytezahl ergeben.');
    return String(numerator / scale);
  }
  function checkedEnvelope(value) {
    if (!value || typeof value !== 'object' || value.engine !== ENGINE || value.version !== 1 || value.max_bytes !== MAX_BYTES) throw Error('Unbekannte native Profilantwort.');
    for (const key of ['profile_json', 'config_json', 'effective_config_json']) {
      if (typeof value[key] !== 'string' || size(value[key]) > MAX_BYTES) throw Error('Ungültiger oder zu großer nativer Profiltext.');
      fields(value[key]);
    }
    const document = fields(value.profile_json);
    if (document.get('version') !== '1' || !document.has('config') || !document.has('required_headers')) throw Error('Unbekanntes Profilformat.');
    const required = JSON.parse(document.get('required_headers'));
    if (!Array.isArray(required) || required.some(name => typeof name !== 'string') || new Set(required).size !== required.length) throw Error('Ungültige erforderliche Headernamen.');
    for (const text of [document.get('config'), value.config_json, value.effective_config_json]) {
      const headers = fields(text).get('custom_headers');
      if (headers !== undefined && fields(headers).size) throw Error('Ein Profil darf keine Headerwerte enthalten.');
    }
    if (!Array.isArray(value.missing_headers) || value.missing_headers.some(name => typeof name !== 'string') || typeof value.requires_targets !== 'boolean' || !Array.isArray(value.normalization)) throw Error('Unvollständige native Profilantwort.');
    return {...value, required_headers: required};
  }
  async function transport(route, body, options = {}) {
    const response = await fetch(route, {method: options.method || (body === undefined ? 'GET' : 'POST'), cache: 'no-store', redirect: 'error',
      signal: options.signal, headers: body === undefined ? {} : {'Content-Type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body)});
    const cap = options.attachment ? MAX_BYTES : MAX_BYTES * 6 + 1048576;
    const declared = response.headers.get('content-length');
    if (declared !== null && (!/^\d+$/.test(declared) || Number(declared) > cap)) throw Error('Die Profilantwort überschreitet das Bytebudget.');
    if (!response.body) throw Error('Die native Profilantwort hat keinen Body.');
    const reader = response.body.getReader(), chunks = []; let total = 0;
    try {
      for (;;) {
        const item = await reader.read(); if (item.done) break;
        total += item.value.byteLength;
        if (total > cap) throw Error('Die Profilantwort überschreitet das Bytebudget.');
        chunks.push(item.value);
      }
    } catch (error) { try { await reader.cancel(); } catch (_) {} throw error; }
    const bytes = new Uint8Array(total); let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    const text = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(bytes);
    if (!response.ok) {
      let failure; try { failure = JSON.parse(text); } catch (_) {}
      const error = Error(typeof failure?.error === 'string' ? failure.error : 'Profilprüfung fehlgeschlagen (HTTP ' + response.status + ').');error.status=response.status;throw error;
    }
    if (options.attachment) {
      if (!response.headers.get('content-disposition')?.includes('attachment')) throw Error('Der Server hat keinen Profil-Download geliefert.');
      return {bytes, text, type: response.headers.get('content-type') || 'application/json'};
    }
    return JSON.parse(text);
  }
  function createController(options = {}) {
    const send = options.transport || transport, storage = options.storage || host.localStorage;
    const changed = options.changed || (() => {}), key = options.key || KEY;
    const state = {active: null, working: null, text: '', busy: false, error: '', restoreError: '', restorePending: false, revision: 0, open: false};
    const secrets = new Map(); let operation = null;
    function invalidate() {
      state.revision++; operation?.controller.abort(); operation = null; state.busy = false;
      options.invalidated?.();
    }
    function notify() { changed(state); }
    function begin() { invalidate(); state.open = true; state.working = state.active; state.text = state.active?.profile_json || ''; state.error = ''; notify(); }
    function close() {
      invalidate();
      if (state.working?.profile_json !== state.active?.profile_json || state.text !== (state.active?.profile_json || '')) secrets.clear();
      state.open = false; state.working = null; state.text = ''; state.error = ''; notify();
    }
    function edited(text) { invalidate(); if (text !== undefined) { state.text = text; secrets.clear(); } state.error = ''; notify(); }
    async function perform(work, accept, guard = () => true, reject = () => {}) {
      invalidate();
      const owner = {revision: state.revision, controller: new AbortController()}; operation = owner;
      const timer = setTimeout(() => owner.controller.abort(), 30000);
      const current = () => operation === owner && state.revision === owner.revision && guard();
      state.busy = true; state.error = ''; notify();
      try {
        const result = await work(owner.controller.signal);
        if (!current()) return null;
        const accepted = accept(result); if (!current()) return null;
        return accepted;
      } catch (error) {
        if (current()) { state.error = error?.name === 'AbortError' ? 'Profilprüfung abgebrochen oder Zeitlimit erreicht.' : error?.message || 'Profilprüfung fehlgeschlagen.'; reject(state.error); notify(); }
        return null;
      } finally {
        clearTimeout(timer);
        if (operation === owner) { operation = null; state.busy = false; notify(); }
      }
    }
    function stage(result, clearSecrets = false) {
      const envelope = checkedEnvelope(result);
      if (clearSecrets) secrets.clear();
      state.working = envelope; state.text = envelope.profile_json; notify(); return envelope;
    }
    const inspect = patches => perform(signal => send('/api/config-profiles/inspect', {profile_text: state.text, patches: patches || []}, {signal}), result => stage(result));
    function importFile(file) {
      return perform(async signal => {
        if (!file || !Number.isSafeInteger(file.size) || file.size < 1 || file.size > MAX_BYTES) throw Error('Eine Profildatei bis 64 MiB auswählen.');
        const bytes = new Uint8Array(await file.slice(0, MAX_BYTES + 1).arrayBuffer());
        if (bytes.byteLength > MAX_BYTES) throw Error('Die Profildatei überschreitet 64 MiB.');
        const text = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(bytes);
        if (signal.aborted) throw new DOMException('Abgebrochen', 'AbortError');
        return send('/api/config-profiles/inspect', {profile_text: text}, {signal});
      }, result => stage(result, true), () => state.open);
    }
    const selected = (id, guard) => perform(signal => send('/api/crawls/' + encodeURIComponent(id) + '/config-profile', undefined, {signal}), result => stage(result, true), guard);
    const defaults = () => perform(signal => send('/api/config-profiles/default', undefined, {signal}), result => stage(result, true));
    const fromConfig = configText => perform(signal => send('/api/config-profiles/from-config', {config_text: configText}, {signal}), result => stage(result, true));
    function save(patches) {
      return perform(signal => send('/api/config-profiles/inspect', {profile_text: state.text, patches: patches || []}, {signal}), result => {
        const envelope = checkedEnvelope(result);
        const stored = JSON.stringify({version: 1, profile_text: envelope.profile_json});
        storage.setItem(key, stored); // One atomic write precedes all active state changes.
        state.active = envelope; state.working = envelope; state.text = envelope.profile_json; state.restoreError = ''; state.restorePending = false; notify(); return envelope;
      }, () => state.open);
    }
    async function restore() {
      return perform(async signal => {
        const stored = storage.getItem(key);
        if (stored === null) return null;
        state.restorePending = true; notify();
        if (size(stored) > MAX_BYTES * 2 + 65536) throw Error('Gespeicherter Profilentwurf ist zu groß.');
        const data = JSON.parse(stored);
        if (!data || Object.keys(data).sort().join(',') !== 'profile_text,version' || data.version !== 1 || typeof data.profile_text !== 'string' || size(data.profile_text) > MAX_BYTES) throw Error('Gespeicherter Profilentwurf ist ungültig.');
        return send('/api/config-profiles/inspect', {profile_text: data.profile_text}, {signal});
      }, result => { if (result === null) return null; const envelope = checkedEnvelope(result); state.active = envelope; state.restorePending = false; secrets.clear(); notify(); return envelope; }, () => true, error => { state.restorePending = false; state.restoreError = error; });
    }
    function exportProfile(patches) {
      return perform(signal => send('/api/config-profiles/export', {profile_text: state.text, patches: patches || []}, {signal, attachment: true}), result => {
        const document = fields(result.text); const config = document.get('config');
        if (document.get('version') !== '1' || !config || !document.has('required_headers')) throw Error('Ungültiger nativer Profil-Download.');
        const headers = fields(config).get('custom_headers');
        if (headers !== undefined && fields(headers).size) throw Error('Ein Profil-Download darf keine Headerwerte enthalten.');
        return result;
      }, () => state.open);
    }
    function header(name, value, present = true) {
      const required = state.working?.required_headers || state.active?.required_headers || [];
      if (!required.includes(name) || typeof value !== 'string') throw Error('Unbekannte Headeranforderung.');
      if (present) secrets.set(name, value); else secrets.delete(name);
      notify();
    }
    function headerValues() {
      const names = state.active?.required_headers || [], missing = names.filter(name => !secrets.has(name));
      if (missing.length) throw Error('Erforderliche Headerwerte fehlen: ' + missing.join(', '));
      return Object.fromEntries(names.map(name => [name, secrets.get(name)]));
    }
    function clearHeaders() { secrets.clear(); notify(); }
    function stageExternal(result) { const envelope=checkedEnvelope(result);if(!state.open)throw Error('Der Einstellungsdialog wurde geschlossen.');invalidate();return stage(envelope,true); }
    return {state, begin, close, edited, invalidate, inspect, importFile, selected, defaults, fromConfig, save, restore, exportProfile, header, headerValues, clearHeaders, stageExternal,
      unresolved: () => (state.working?.required_headers || state.active?.required_headers || []).filter(name => !secrets.has(name)),
      hasHeader: name => secrets.has(name), headerValue: name => secrets.get(name) ?? ''};
  }
  const CATALOG_ENGINE='open-seo-spider/config-profile-catalog-v1',CATALOG_ROUTE='/api/config-profile-catalog';
  const CATALOG_LIMITS={max_entries:128,max_name_bytes:256,max_profile_bytes:MAX_BYTES,max_total_profile_bytes:268435456};
  const uuid=value=>typeof value==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value);
  const digest=value=>typeof value==='string'&&/^[0-9a-f]{64}$/.test(value);
  function objectKeys(value,keys) { return value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).sort().join(',')===keys.slice().sort().join(','); }
  function catalogName(value) {
    if(typeof value!=='string'||!value.length||size(value)>256||/^\p{White_Space}*$/u.test(value)||/\p{Cc}/u.test(value))throw Error('Profilname: 1 bis 256 UTF-8-Bytes, nicht leer und ohne Steuerzeichen. Leerzeichen im Namen bleiben unverändert.');
    return value;
  }
  function checkedCatalogEntry(value) {
    if(!objectKeys(value,['id','name','revision','profile_sha256','profile_bytes','created_at','updated_at'])||!uuid(value.id)||typeof value.revision!=='string'||!/^\d{1,19}$/.test(value.revision)||value.revision[0]==='0'||BigInt(value.revision)>9223372036854775807n||!digest(value.profile_sha256)||!Number.isSafeInteger(value.profile_bytes)||value.profile_bytes<1||value.profile_bytes>MAX_BYTES)throw Error('Ungültige Metadaten der Profilbibliothek.');
    catalogName(value.name);
    for(const key of ['created_at','updated_at'])if(typeof value[key]!=='string'||size(value[key])>64||!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/.test(value[key])||!Number.isFinite(Date.parse(value[key])))throw Error('Ungültiger Zeitstempel der Profilbibliothek.');
    return {...value};
  }
  function checkedCatalogList(value) {
    if(!objectKeys(value,['engine','version','scope','entries','limits'])||value.engine!==CATALOG_ENGINE||value.version!==1||value.scope!=='selected_application_data_directory'||!objectKeys(value.limits,Object.keys(CATALOG_LIMITS))||Object.entries(CATALOG_LIMITS).some(([key,limit])=>value.limits[key]!==limit)||!Array.isArray(value.entries)||value.entries.length>128)throw Error('Unbekannte oder unvollständige Profilbibliothek.');
    const entries=value.entries.map(checkedCatalogEntry),ids=new Set(),names=new Set();let bytes=0;
    for(const entry of entries){if(ids.has(entry.id)||names.has(entry.name))throw Error('Doppelte Einträge in der Profilbibliothek.');ids.add(entry.id);names.add(entry.name);bytes+=entry.profile_bytes;}
    if(bytes>CATALOG_LIMITS.max_total_profile_bytes)throw Error('Die Profilbibliothek überschreitet ihr Gesamtbudget.');return entries;
  }
  async function textSha256(text) { return [...new Uint8Array(await host.crypto.subtle.digest('SHA-256',new TextEncoder().encode(text)))].map(byte=>byte.toString(16).padStart(2,'0')).join(''); }
  function createCatalogController(options) {
    const profiles=options.profiles,send=options.transport||transport,changed=options.changed||(()=>{});
    const state={entries:[],selectedId:'',loaded:false,busy:false,error:'',status:'Noch nicht geladen. Profile werden im gewählten lokalen App-Datenverzeichnis gespeichert.',revision:0};let operation=null,installing=false;
    const selected=()=>state.entries.find(entry=>entry.id===state.selectedId)||null;
    const notify=()=>changed(state);
    function invalidate() { if(installing)return;state.revision++;operation?.controller.abort();operation=null;state.busy=false; }
    function edited() { invalidate();state.error='';notify(); }
    function select(id) { invalidate();state.error='';if(id!==''&&!state.entries.some(entry=>entry.id===id)){state.error='Den Bibliothekseintrag erneut laden.';notify();return;}state.selectedId=id;notify(); }
    async function run(work,accept) {
      invalidate();const owner={revision:state.revision,profileRevision:profiles.state.revision,controller:new AbortController()};operation=owner;
      const current=()=>operation===owner&&state.revision===owner.revision&&profiles.state.revision===owner.profileRevision&&profiles.state.open;
      const assertCurrent=()=>{if(!current()||owner.controller.signal.aborted)throw new DOMException('Überholte Bibliotheksaktion','AbortError');};
      const timer=setTimeout(()=>owner.controller.abort(),30000);state.busy=true;state.error='';notify();
      try{assertCurrent();const result=await work(owner.controller.signal,assertCurrent);assertCurrent();const accepted=accept(result);notify();return accepted;}
      catch(error){if(current()){state.error=error?.status===409?'Konflikt: Der Eintrag wurde geändert oder der Name ist bereits vergeben. Gespeicherte Profile erneut laden. '+error.message:error?.name==='AbortError'?'Bibliotheksaktion abgebrochen oder Zeitlimit erreicht. Vor erneutem Ändern die Liste laden.':error?.message||'Profilbibliothek nicht verfügbar.';notify();}return null;}
      finally{clearTimeout(timer);if(operation===owner){operation=null;state.busy=false;notify();}}
    }
    function candidateEntries(entry) { const index=state.entries.findIndex(item=>item.id===entry.id),entries=index<0?[...state.entries,entry]:state.entries.map((item,i)=>i===index?entry:item);return checkedCatalogList({engine:CATALOG_ENGINE,version:1,scope:'selected_application_data_directory',entries,limits:CATALOG_LIMITS}); }
    function upsert(entry) { state.entries=candidateEntries(entry);state.selectedId=entry.id; }
    const requireSelected=()=>{const entry=selected();if(!entry)throw Error('Zuerst einen gespeicherten Eintrag auswählen.');return {...entry};};
    const cas=entry=>({expected_revision:entry.revision,expected_sha256:entry.profile_sha256});
    async function inspected(preparation,signal,assertCurrent) {
      let envelope;
      if(typeof preparation.profile_text==='string')envelope=checkedEnvelope(await send('/api/config-profiles/inspect',{profile_text:preparation.profile_text,patches:preparation.patches||[]},{signal}));
      else if(typeof preparation.config_text==='string')envelope=checkedEnvelope(await send('/api/config-profiles/from-config',{config_text:preparation.config_text},{signal}));
      else throw Error('Zuerst einen vollständigen Profilentwurf prüfen.');
      assertCurrent();const sha256=await textSha256(envelope.profile_json);assertCurrent();return {text:envelope.profile_json,sha256,bytes:size(envelope.profile_json)};
    }
    const list=()=>run(async signal=>checkedCatalogList(await send(CATALOG_ROUTE,undefined,{signal})),entries=>{state.entries=entries;state.loaded=true;if(!entries.some(entry=>entry.id===state.selectedId))state.selectedId='';state.status=entries.length+' gespeicherte Profile im gewählten App-Datenverzeichnis.';return entries;});
    const load=()=>{profiles.invalidate();return run(async signal=>{
      const target=requireSelected(),reply=await send(CATALOG_ROUTE+'/'+target.id,undefined,{signal});if(!objectKeys(reply,['entry','profile']))throw Error('Ungültige geladene Profilantwort.');
      const entry=checkedCatalogEntry(reply.entry),profile=checkedEnvelope(reply.profile);if(entry.id!==target.id||BigInt(entry.revision)<BigInt(target.revision)||entry.profile_bytes!==size(profile.profile_json)||entry.profile_sha256!==await textSha256(profile.profile_json))throw Error('Profiltext und Bibliotheksmetadaten stimmen nicht überein.');return {entry,profile};
    },({entry,profile})=>{const entries=candidateEntries(entry);installing=true;let staged;try{staged=profiles.stageExternal(profile);}finally{installing=false;}state.entries=entries;state.selectedId=entry.id;state.status='„'+entry.name+'“ ist nur in den Dialogentwurf geladen. Erst Einstellungen übernehmen aktiviert es; erforderliche Headerwerte neu eingeben.';return staged;});};
    const write=(name,preparation,replace)=>run(async(signal,assertCurrent)=>{
      catalogName(name);const target=replace?requireSelected():null,prepared=await inspected(preparation,signal,assertCurrent);assertCurrent();
      const body={...(target?cas(target):{}),name,profile_text:prepared.text},reply=await send(CATALOG_ROUTE+(target?'/'+target.id:''),body,{signal,method:target?'PUT':'POST'});
      if(!objectKeys(reply,['entry']))throw Error('Ungültige Antwort beim Speichern in der Bibliothek.');const entry=checkedCatalogEntry(reply.entry);
      if(entry.name!==name||entry.profile_sha256!==prepared.sha256||entry.profile_bytes!==prepared.bytes||target&&(entry.id!==target.id||BigInt(entry.revision)<=BigInt(target.revision)||entry.created_at!==target.created_at)||!target&&state.entries.some(item=>item.id===entry.id))throw Error('Gespeicherte Profilmetadaten passen nicht zur bestätigten Aktion.');return entry;
    },entry=>{upsert(entry);state.status='„'+entry.name+'“ wurde in der nativen Bibliothek gespeichert. Aktiver Entwurf und nächster Crawl bleiben unverändert.';return entry;});
    const rename=name=>run(async signal=>{
      catalogName(name);const target=requireSelected(),reply=await send(CATALOG_ROUTE+'/'+target.id+'/rename',{...cas(target),name},{signal});if(!objectKeys(reply,['entry']))throw Error('Ungültige Umbenennungsantwort.');const entry=checkedCatalogEntry(reply.entry);
      if(entry.id!==target.id||entry.name!==name||entry.profile_sha256!==target.profile_sha256||entry.profile_bytes!==target.profile_bytes||entry.created_at!==target.created_at||BigInt(entry.revision)<=BigInt(target.revision))throw Error('Umbenennungsmetadaten passen nicht zum gewählten Eintrag.');return entry;
    },entry=>{upsert(entry);state.status='Bibliothekseintrag umbenannt. Aktiver Entwurf und Crawl-Konfiguration bleiben unverändert.';return entry;});
    const remove=()=>run(async signal=>{const target=requireSelected(),reply=await send(CATALOG_ROUTE+'/'+target.id,cas(target),{signal,method:'DELETE'});if(!objectKeys(reply,['deleted_id'])||reply.deleted_id!==target.id)throw Error('Ungültige Löschbestätigung.');return target;},target=>{state.entries=state.entries.filter(item=>item.id!==target.id);state.selectedId='';state.status='Bibliothekseintrag gelöscht. Bereits geladener Entwurf und gespeicherte Crawl-Konfiguration bleiben unverändert.';return target;});
    return {state,list,load,create:(name,preparation)=>write(name,preparation,false),overwrite:(name,preparation)=>write(name,preparation,true),rename,remove,select,selected,invalidate,edited};
  }
  host.ConfigProfiles = {KEY, ENGINE, MAX_BYTES, fields, at, toMiB, fromMiB, createController, checkedEnvelope, CATALOG_ENGINE, CATALOG_LIMITS,checkedCatalogEntry,checkedCatalogList,catalogName,createCatalogController};
})(typeof window === 'object' ? window : globalThis);
