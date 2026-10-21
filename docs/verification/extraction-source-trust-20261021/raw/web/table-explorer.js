'use strict';
(function (host, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  else host.TableExplorer = api;
})(typeof globalThis === 'object' ? globalThis : this, function () {
  // Browser column preferences belong to this origin, project ID and section.
  // They are not a native database or a portable crawl/project format.
  const LIMITS = Object.freeze({ maxColumns: 128, maxRules: 16, maxTextBytes: 4096, maxPreferencesBytes: 65536, maxKeyBytes: 256, maxProjectBytes: 256, maxSectionBytes: 128 });
  const EXPRESSION_LIMITS = Object.freeze({ maxLeaves: 16, maxNodes: 31, maxGroupDepth: 4, maxExpressionBytes: 131072 });
  const encoder = new TextEncoder();
  const collator = new Intl.Collator('de-DE', { numeric: true, sensitivity: 'base' });
  const identityCollator = new Intl.Collator('de-DE', { numeric: true, sensitivity: 'variant' });
  const own = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
  const object = value => Boolean(value && typeof value === 'object' && !Array.isArray(value));
  const success = value => ({ ok: true, value });
  function reject(code, message) { const error = new Error(message); error.code = code; throw error; }
  function result(action) {
    try { return success(action()); }
    catch (error) {
      let code = 'evaluation_failed', message = 'Die Tabellenansicht konnte nicht ausgewertet werden.';
      try { if (error && typeof error.code === 'string') code = error.code; message = String(error && error.message || error); } catch (_) { /* A throwing getter must still produce a Result. */ }
      return { ok: false, error: { code, message } };
    }
  }
  function keys(value, allowed, required = []) {
    if (!object(value) || Object.keys(value).some(key => !allowed.includes(key)) || required.some(key => !own(value, key))) reject('invalid_shape', 'Die Tabellenansicht hat ein ungültiges Format.');
  }
  function boundedText(value, maximum, code, allowEmpty = true) {
    if (typeof value !== 'string' || value.length > maximum || encoder.encode(value).length > maximum || (!allowEmpty && !value.trim())) reject(code, 'Der Text fehlt oder überschreitet die Grenze.');
    return value;
  }
  function normalizeText(value) {
    if (typeof value !== 'string') throw new TypeError('Text muss eine Zeichenfolge sein.');
    return value.normalize('NFC').trim().toLocaleLowerCase('de-DE');
  }
  function classify(raw, type = 'auto') {
    if (!['auto', 'text', 'number'].includes(type)) throw new TypeError('Unbekannter Spaltentyp.');
    if (raw == null || typeof raw === 'number' && !Number.isFinite(raw) || !['string', 'number', 'boolean'].includes(typeof raw)) return { kind: 'unknown', value: null, search: '' };
    if (typeof raw === 'string' && !raw.trim()) return { kind: 'empty', value: '', search: '' };
    if (type === 'number' && typeof raw !== 'number') return { kind: 'unknown', value: null, search: '' };
    if (type === 'text' || typeof raw === 'string') return { kind: 'text', value: String(raw).normalize('NFC'), search: normalizeText(String(raw)) };
    return { kind: typeof raw === 'number' ? 'number' : 'boolean', value: raw, search: String(raw) };
  }
  function schemaFor(columns) {
    if (!Array.isArray(columns) || !columns.length || columns.length > LIMITS.maxColumns) reject('invalid_columns', 'Die Spaltenliste ist leer oder zu groß.');
    const byKey = new Map();
    for (const column of columns) {
      if (!object(column)) reject('invalid_columns', 'Eine Spaltenbeschreibung fehlt.');
      const key = boundedText(column.key, LIMITS.maxKeyBytes, 'invalid_column_key', false);
      if (byKey.has(key)) reject('duplicate_column_key', 'Ein Spaltenschlüssel kommt mehrfach vor.');
      const type = column.type === undefined ? 'auto' : column.type;
      if (!['auto', 'text', 'number'].includes(type) || own(column, 'mandatory') && typeof column.mandatory !== 'boolean') reject('invalid_columns', 'Ungültiger Spaltentyp oder Pflichtspaltenwert.');
      const getter = own(column, 'rawValue') ? column.rawValue : column.value;
      if (typeof getter !== 'function') reject('invalid_columns', 'Eine Spalte benötigt einen Rohwert-Leser.');
      const displayGetter = typeof column.value === 'function' && column.value !== getter ? column.value : null;
      byKey.set(key, { descriptor: column, type, getter, displayGetter });
    }
    return byKey;
  }
  function projection(columns, byKey, visibleKeys) {
    if (!Array.isArray(visibleKeys) || visibleKeys.length > LIMITS.maxColumns) reject('invalid_projection', 'Ungültige sichtbare Spaltenliste.');
    const seen = new Set();
    for (const key of visibleKeys) {
      if (typeof key !== 'string' || !byKey.has(key)) reject('unknown_column', 'Eine ausgewählte Spalte ist nicht mehr verfügbar.');
      if (seen.has(key)) reject('duplicate_visible_key', 'Eine sichtbare Spalte kommt mehrfach vor.');
      seen.add(key);
    }
    const selected = visibleKeys.map(key => byKey.get(key).descriptor);
    for (const column of columns) if (column.mandatory && !seen.has(column.key)) { seen.add(column.key); selected.push(column); }
    if (!selected.length) reject('empty_projection', 'Mindestens eine Spalte muss sichtbar bleiben.');
    return selected;
  }
  function projectColumns(columns, visibleKeys) { return result(() => projection(columns, schemaFor(columns), visibleKeys)); }
  function ruleFor(rule, byKey) {
    if (!object(rule) || typeof rule.key !== 'string' || !byKey.has(rule.key)) reject('unknown_filter_column', 'Die Filterspalte ist nicht verfügbar.');
    const allowed = { contains: ['key', 'op', 'text'], equals: ['key', 'op', 'text'], min: ['key', 'op', 'min'], max: ['key', 'op', 'max'], range: ['key', 'op', 'min', 'max'], known: ['key', 'op'], unknown: ['key', 'op'], empty: ['key', 'op'] };
    if (!own(allowed, rule.op)) reject('invalid_operator', 'Unbekannter Filteroperator.');
    keys(rule, allowed[rule.op], allowed[rule.op]);
    if (['contains', 'equals'].includes(rule.op)) return { ...rule, text: normalizeText(boundedText(rule.text, LIMITS.maxTextBytes, 'filter_text_limit')) };
    if (['min', 'max', 'range'].includes(rule.op)) {
      if (byKey.get(rule.key).type === 'text') reject('incompatible_filter', 'Zahlengrenzen benötigen eine numerische Spalte.');
      for (const field of ['min', 'max']) if (own(rule, field) && (typeof rule[field] !== 'number' || !Number.isFinite(rule[field]))) reject('invalid_numeric_bound', 'Zahlengrenzen müssen endliche Zahlen sein.');
      if (rule.op === 'range' && rule.min > rule.max) reject('invalid_numeric_range', 'Die Untergrenze liegt über der Obergrenze.');
    }
    return { ...rule };
  }
  function matches(value, rule) {
    switch (rule.op) {
      case 'unknown': return value.kind === 'unknown';
      case 'empty': return value.kind === 'empty';
      case 'known': return value.kind !== 'unknown' && value.kind !== 'empty';
      case 'contains': return value.kind !== 'unknown' && value.search.includes(rule.text);
      case 'equals': return value.kind !== 'unknown' && value.search === rule.text;
      case 'min': return value.kind === 'number' && value.value >= rule.min;
      case 'max': return value.kind === 'number' && value.value <= rule.max;
      case 'range': return value.kind === 'number' && value.value >= rule.min && value.value <= rule.max;
      default: return false;
    }
  }
  function compareValue(a, b, direction) {
    const rank = value => value.kind === 'unknown' ? 2 : value.kind === 'empty' ? 1 : 0;
    const placement = rank(a) - rank(b);
    if (placement) return placement;
    if (rank(a)) return 0;
    if (a.kind === 'number' && b.kind === 'number') return (a.value < b.value ? -1 : a.value > b.value ? 1 : 0) * direction;
    if (a.kind === 'boolean' && b.kind === 'boolean') return (Number(a.value) - Number(b.value)) * direction;
    if (a.kind !== b.kind) return ({ number: 0, boolean: 1, text: 2 }[a.kind] - { number: 0, boolean: 1, text: 2 }[b.kind]) * direction;
    return collator.compare(a.value, b.value) * direction;
  }
  function compile(columns, view, identity = {}) {
    return result(() => {
      const byKey = schemaFor(columns);
      keys(view, ['search', 'rules', 'sort', 'visibleKeys'], ['search', 'rules', 'sort', 'visibleKeys']);
      const search = normalizeText(boundedText(view.search, LIMITS.maxTextBytes, 'search_text_limit'));
      if (!Array.isArray(view.rules) || view.rules.length > LIMITS.maxRules) reject('rule_limit', 'Zu viele Filterregeln.');
      const rules = view.rules.map(rule => ruleFor(rule, byKey));
      keys(view.sort, ['key', 'direction'], ['key', 'direction']);
      if (!byKey.has(view.sort.key) || ![1, -1].includes(view.sort.direction)) reject('invalid_sort', 'Ungültige Sortierspalte oder Richtung.');
      const sortKey = view.sort.key, direction = view.sort.direction;
      const projected = projection(columns, byKey, view.visibleKeys);
      keys(identity, ['identity', 'ordinal']);
      if (own(identity, 'identity') && typeof identity.identity !== 'function' || own(identity, 'ordinal') && typeof identity.ordinal !== 'function') reject('invalid_identity', 'Ungültiger Zeilenidentitäts-Leser.');
      const getIdentity = identity.identity || (row => row && row.url);
      const getOrdinal = identity.ordinal || (() => null);
      function rows(inputRows) {
        return result(() => {
          if (!Array.isArray(inputRows)) reject('invalid_rows', 'Die Tabellenzeilen sind keine Liste.');
          const selected = [];
          for (let index = 0; index < inputRows.length; index++) {
            const row = inputRows[index], values = new Map();
            const value = key => { if (!values.has(key)) { const column = byKey.get(key); values.set(key, classify(column.getter(row), column.type)); } return values.get(key); };
            if (rules.some(rule => !matches(value(rule.key), rule))) continue;
            if (search && !Array.from(byKey.keys()).some(key => {
              const raw = value(key), column = byKey.get(key);
              if (raw.kind !== 'unknown' && raw.search.includes(search)) return true;
              return column.displayGetter !== null && classify(column.displayGetter(row)).search.includes(search);
            })) continue;
            const rawIdentity = getIdentity(row);
            const rowIdentity = typeof rawIdentity === 'string' ? rawIdentity.normalize('NFC') : typeof rawIdentity === 'number' && Number.isFinite(rawIdentity) ? String(rawIdentity) : '';
            const rawOrdinal = getOrdinal(row), ordinal = Number.isSafeInteger(rawOrdinal) && rawOrdinal >= 0 ? rawOrdinal : index;
            selected.push({ row, index, identity: rowIdentity, ordinal, sort: value(sortKey) });
          }
          selected.sort((a, b) => compareValue(a.sort, b.sort, direction) || identityCollator.compare(a.identity, b.identity) || a.ordinal - b.ordinal || a.index - b.index);
          return selected.map(entry => entry.row);
        });
      }
      return { columns: projected, rows };
    });
  }
  // V2 declarations are copied without invoking accessors. Existing V1 APIs
  // retain their own validation and limits; no expression cap is added to them.
  function expressionObject(value, allowed, required=allowed) {
    if(!object(value))reject('invalid_expression_shape','Ein Filterknoten muss ein Datenobjekt sein.');
    const prototype=Object.getPrototypeOf(value);
    if(prototype!==Object.prototype&&prototype!==null)reject('invalid_expression_shape','Filterdaten benötigen ein einfaches Objekt.');
    const fields=Reflect.ownKeys(value);
    if(fields.some(key=>typeof key!=='string'||!allowed.includes(key))||required.some(key=>!own(value,key)))reject('invalid_expression_shape','Unbekannte oder fehlende Filterdaten.');
    const output={};
    for(const key of fields){const descriptor=Object.getOwnPropertyDescriptor(value,key);if(!descriptor||!own(descriptor,'value')||!descriptor.enumerable)reject('invalid_expression_shape','Filterdaten dürfen keine Accessors oder versteckten Felder enthalten.');Object.defineProperty(output,key,{value:descriptor.value,enumerable:true,writable:true,configurable:true});}
    return output;
  }
  function expressionArray(value,maximum,code='invalid_expression_array') {
    if(!Array.isArray(value))reject(code,'Eine dichte Datenliste ist erforderlich.');
    const descriptor=Object.getOwnPropertyDescriptor(value,'length'),length=descriptor&&descriptor.value;
    if(!Number.isSafeInteger(length)||length<0||length>maximum)reject(code,'Die Datenliste überschreitet die Grenze.');
    const fields=Reflect.ownKeys(value);
    if(fields.length!==length+1||fields.some(key=>typeof key!=='string'||key!=='length'&&!/^(0|[1-9][0-9]*)$/.test(key)))reject(code,'Listen benötigen ausschließlich eigene dichte Datenindizes.');
    const output=[];for(let index=0;index<length;index++){const cell=Object.getOwnPropertyDescriptor(value,String(index));if(!cell||!own(cell,'value')||!cell.enumerable)reject(code,'Fehlende, geerbte oder berechnete Listeneinträge sind nicht erlaubt.');output.push(cell.value);}return output;
  }
  function frozenExpression(value){if(value&&typeof value==='object'){for(const child of Object.values(value))frozenExpression(child);Object.freeze(value);}return value;}
  function expressionSchema(columns){
    const copied=expressionArray(columns,LIMITS.maxColumns,'invalid_columns');
    for(const column of copied){
      if(!object(column))reject('invalid_columns','Eine Spaltenbeschreibung fehlt.');
      for(const key of ['key','type','mandatory','rawValue','value'])if(key in column){const descriptor=Object.getOwnPropertyDescriptor(column,key);if(!descriptor||!own(descriptor,'value'))reject('invalid_columns','Spaltenmetadaten und Leser müssen eigene Datenwerte sein.');}
    }
    return{columns:copied,byKey:schemaFor(copied)};
  }
  function expressionFor(byKey,expression) {
    if(expression===null)return {declared:null,compiled:null};
    const active=new Set();let nodes=0,leaves=0;
    const leafFields={contains:['key','op','text'],equals:['key','op','text'],min:['key','op','min'],max:['key','op','max'],range:['key','op','min','max'],known:['key','op'],unknown:['key','op'],empty:['key','op']};
    function visit(node,groupDepth){
      if(node===null||!object(node))reject('invalid_expression_shape','Nur der gesamte Filter darf leer sein.');
      if(active.has(node))reject('expression_cycle','Zyklische Filterdaten sind nicht erlaubt.');
      if(++nodes>EXPRESSION_LIMITS.maxNodes)reject('expression_node_limit','Zu viele Filterknoten.');
      active.add(node);
      try{
        const kindDescriptor=Object.getOwnPropertyDescriptor(node,'kind');
        if(!kindDescriptor||!own(kindDescriptor,'value'))reject('invalid_expression_shape','Der Knotentyp muss ein eigener Datenwert sein.');
        const kind=kindDescriptor.value;
        if(kind==='rule'){
          if(++leaves>EXPRESSION_LIMITS.maxLeaves)reject('expression_leaf_limit','Zu viele einzelne Filterregeln.');
          const checked=expressionObject(node,['kind','rule']),opDescriptor=checked.rule&&Object.getOwnPropertyDescriptor(checked.rule,'op');
          if(!opDescriptor||!own(opDescriptor,'value')||!own(leafFields,opDescriptor.value))reject('invalid_operator','Unbekannter Filteroperator.');
          const rule=expressionObject(checked.rule,leafFields[opDescriptor.value]);
          boundedText(rule.key,LIMITS.maxKeyBytes,'invalid_column_key',false);
          const compiled=ruleFor(rule,byKey||new Map([[rule.key,{type:'auto'}]]));
          return {declared:{kind:'rule',rule},compiled:{kind:'rule',rule:compiled}};
        }
        if(kind!=='all'&&kind!=='any')reject('invalid_expression_kind','Unbekannter Filtergruppentyp.');
        const checked=expressionObject(node,['kind','children']),nextDepth=groupDepth+1;
        if(nextDepth>EXPRESSION_LIMITS.maxGroupDepth)reject('expression_depth_limit','Filtergruppen sind zu tief verschachtelt.');
        const children=expressionArray(checked.children,EXPRESSION_LIMITS.maxNodes);
        if(!children.length)reject('empty_expression_group','Eine Filtergruppe benötigt mindestens einen Knoten.');
        const prepared=children.map(child=>visit(child,nextDepth));
        return {declared:{kind,children:prepared.map(child=>child.declared)},compiled:{kind,children:prepared.map(child=>child.compiled)}};
      }finally{active.delete(node);}
    }
    const prepared=visit(expression,0),raw=JSON.stringify(prepared.declared);
    if(encoder.encode(raw).length>EXPRESSION_LIMITS.maxExpressionBytes)reject('expression_byte_limit','Die Filterdefinition überschreitet 128 KiB.');
    frozenExpression(prepared.declared);return prepared;
  }
  function validateExpression(columns,expression){return result(()=>expressionFor(expressionSchema(columns).byKey,expression).declared);}
  function fromRules(rules){return result(()=>{const copied=expressionArray(rules,LIMITS.maxRules),expression=copied.length===0?null:copied.length===1?{kind:'rule',rule:copied[0]}:{kind:'all',children:copied.map(rule=>({kind:'rule',rule}))};return expressionFor(null,expression).declared;});}
  function expressionMatches(node,value){if(node===null)return true;if(node.kind==='rule')return matches(value(node.rule.key),node.rule);return node.kind==='all'?node.children.every(child=>expressionMatches(child,value)):node.children.some(child=>expressionMatches(child,value));}
  function compileExpression(columns,view,identity={}) {
    return result(()=>{
      const checkedSchema=expressionSchema(columns),byKey=checkedSchema.byKey,declaration=expressionObject(view,['version','search','expression','sort','visibleKeys']);
      if(declaration.version!==2)reject('unsupported_expression_version','Die Filterdefinition benötigt Version 2.');
      const search=normalizeText(boundedText(declaration.search,LIMITS.maxTextBytes,'search_text_limit')),prepared=expressionFor(byKey,declaration.expression),sort=expressionObject(declaration.sort,['key','direction']);
      if(!byKey.has(sort.key)||![1,-1].includes(sort.direction))reject('invalid_sort','Ungültige Sortierspalte oder Richtung.');
      const projected=projection(checkedSchema.columns,byKey,expressionArray(declaration.visibleKeys,LIMITS.maxColumns)),callbacks=expressionObject(identity,['identity','ordinal'],[]);
      if(own(callbacks,'identity')&&typeof callbacks.identity!=='function'||own(callbacks,'ordinal')&&typeof callbacks.ordinal!=='function')reject('invalid_identity','Ungültiger Zeilenidentitäts-Leser.');
      const getIdentity=callbacks.identity||(row=>row&&row.url),getOrdinal=callbacks.ordinal||(()=>null);
      function rows(inputRows){return result(()=>{
        if(!Array.isArray(inputRows))reject('invalid_rows','Die Tabellenzeilen sind keine Liste.');
        const selected=[];
        for(let index=0;index<inputRows.length;index++){
          const row=inputRows[index],values=new Map(),value=key=>{if(!values.has(key)){const column=byKey.get(key);values.set(key,classify(column.getter(row),column.type));}return values.get(key);};
          if(!expressionMatches(prepared.compiled,value))continue;
          if(search&&!Array.from(byKey.keys()).some(key=>{const raw=value(key),column=byKey.get(key);if(raw.kind!=='unknown'&&raw.search.includes(search))return true;return column.displayGetter!==null&&classify(column.displayGetter(row)).search.includes(search);} ))continue;
          const rawIdentity=getIdentity(row),rowIdentity=typeof rawIdentity==='string'?rawIdentity.normalize('NFC'):typeof rawIdentity==='number'&&Number.isFinite(rawIdentity)?String(rawIdentity):'',rawOrdinal=getOrdinal(row),ordinal=Number.isSafeInteger(rawOrdinal)&&rawOrdinal>=0?rawOrdinal:index;
          selected.push({row,index,identity:rowIdentity,ordinal,sort:value(sort.key)});
        }
        selected.sort((a,b)=>compareValue(a.sort,b.sort,sort.direction)||identityCollator.compare(a.identity,b.identity)||a.ordinal-b.ordinal||a.index-b.index);return selected.map(entry=>entry.row);
      });}
      return{columns:projected,rows};
    });
  }
  function contextFor(context) {
    keys(context, ['projectId', 'section'], ['projectId', 'section']);
    boundedText(context.projectId, LIMITS.maxProjectBytes, 'invalid_context', false);
    boundedText(context.section, LIMITS.maxSectionBytes, 'invalid_context', false);
    if (/[\u0000-\u001f\u007f-\u009f]/u.test(context.projectId + context.section)) reject('invalid_context', 'Ungültiger Projekt- oder Bereichsschlüssel.');
    return context;
  }
  function keyFor(context) { contextFor(context); return 'open-seo-spider.table-columns.v1:' + encodeURIComponent(context.projectId) + ':' + encodeURIComponent(context.section); }
  function storageFor(storage, method) {
    if (!storage || typeof storage[method] !== 'function') reject('storage_unavailable', 'Browserspeicher ist nicht verfügbar.');
    return storage;
  }
  function preferences(columns, visibleKeys) { return { visibleKeys: projection(columns, schemaFor(columns), visibleKeys).map(column => column.key) }; }
  function load(storage, context, columns) {
    const loaded = result(() => {
      const key = keyFor(context), target = storageFor(storage, 'getItem');
      let raw; try { raw = target.getItem(key); } catch (_) { reject('storage_unavailable', 'Die gespeicherten Spalten konnten nicht gelesen werden.'); }
      if (raw === null) return null;
      boundedText(raw, LIMITS.maxPreferencesBytes, 'preferences_byte_limit');
      let document; try { document = JSON.parse(raw); } catch (_) { reject('corrupt_preferences', 'Die gespeicherten Spalten sind kein gültiges JSON.'); }
      keys(document, ['version', 'projectId', 'section', 'visibleKeys'], ['version', 'projectId', 'section', 'visibleKeys']);
      if (document.version !== 1 || document.projectId !== context.projectId || document.section !== context.section) reject('preferences_context_mismatch', 'Die gespeicherten Spalten gehören zu einem anderen Format, Projekt oder Bereich.');
      return preferences(columns, document.visibleKeys);
    });
    return loaded.ok ? { ...loaded, missing: loaded.value === null } : loaded;
  }
  function save(storage, context, columns, value) {
    return result(() => {
      const key = keyFor(context); keys(value, ['visibleKeys'], ['visibleKeys']);
      const selected = preferences(columns, value.visibleKeys);
      const raw = JSON.stringify({ version: 1, projectId: context.projectId, section: context.section, visibleKeys: selected.visibleKeys });
      boundedText(raw, LIMITS.maxPreferencesBytes, 'preferences_byte_limit');
      const target = storageFor(storage, 'setItem');
      try { target.setItem(key, raw); } catch (_) { reject('storage_unavailable', 'Die Spalten konnten nicht gespeichert werden.'); }
      return selected;
    });
  }
  function reset(storage, context) {
    return result(() => { const key = keyFor(context), target = storageFor(storage, 'removeItem'); try { target.removeItem(key); } catch (_) { reject('storage_unavailable', 'Die Spalten konnten nicht zurückgesetzt werden.'); } return null; });
  }
  return Object.freeze({ LIMITS, EXPRESSION_LIMITS, normalizeText, classify, projectColumns, compile, validateExpression, compileExpression, fromRules, keyFor, load, save, reset });
});
