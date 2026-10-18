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
    const response = await fetch(route, {method: body === undefined ? 'GET' : 'POST', cache: 'no-store', redirect: 'error',
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
      throw Error(typeof failure?.error === 'string' ? failure.error : 'Profilprüfung fehlgeschlagen (HTTP ' + response.status + ').');
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
    return {state, begin, close, edited, invalidate, inspect, importFile, selected, defaults, fromConfig, save, restore, exportProfile, header, headerValues, clearHeaders,
      unresolved: () => (state.working?.required_headers || state.active?.required_headers || []).filter(name => !secrets.has(name)),
      hasHeader: name => secrets.has(name), headerValue: name => secrets.get(name) ?? ''};
  }
  host.ConfigProfiles = {KEY, ENGINE, MAX_BYTES, fields, at, toMiB, fromMiB, createController, checkedEnvelope};
})(typeof window === 'object' ? window : globalThis);
