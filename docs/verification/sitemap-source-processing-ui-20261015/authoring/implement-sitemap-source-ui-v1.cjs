const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const repo='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-source-processing-ui-v1',sha=b=>createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw Error(m);};
const appPath=path.join(repo,'web/app.js'),indexPath=path.join(repo,'web/index.html'),appBefore=fs.readFileSync(appPath),indexBefore=fs.readFileSync(indexPath);let app=appBefore.toString(),html=indexBefore.toString();
insist(execFileSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}).trim()==='f43efc17d1393645a9eb8c8b8510743f3dfedb5c','source basis changed');
insist(sha(indexBefore)==='f5f494781f91029f1b64ace3c0f1ddcba742014690e0d4ee0dbbef53ebf79cff','index before changed');
const originalApp=execFileSync('git',['show','HEAD:web/app.js'],{cwd:repo});insist(sha(originalApp)==='84df9b724802d86d69262d98dcee10e22d465c7a6583fc034b438df452bb0bb9','baseline app changed');
let inverse=app.replace(/\r\n/g,'\n');inverse=inverse.replace("      sitemap_source_processing_enabled: false, sitemap_source_max_post_http_entity_bytes: '8388608',\n      sitemap_source_max_decoded_xml_bytes: '8388608', sitemap_source_max_xml_events: '500000',\n",'').replace('    syncSitemapSourceProcessingSettings();\n','');insist(inverse===originalApp.toString().replace(/\r\n/g,'\n'),'only declared initial UI edits allowed');
function edit(before,after){insist(app.split(before).length===2,'exact app edit missing/duplicated '+before.slice(0,90));app=app.replace(before,after);}
edit('const advancedText = { include_patterns_text:', 'const advancedText = { ...readSitemapSourceProcessingSettings(form), include_patterns_text:');
edit('...parseResourceDiscovery(settings) };','...parseResourceDiscovery(settings), ...parseSitemapSourceProcessing(settings) };');
edit("      payload.renderer = state.settings.javascript_rendering", "      for (const key of ['sitemap_source_processing_enabled', ...sitemapSourceFields.map(field => field.setting)]) delete payload[key];\r\n      payload.renderer = state.settings.javascript_rendering");
edit("      $('settings-error').textContent = error.message;", "      if (form.elements.sitemap_source_processing_enabled.checked) $('sitemap-source-processing-settings').open = true;\r\n      $('settings-error').textContent = error.message;");
edit("        Object.assign(state.settings,{resource_discovery_enabled:saved.resource_discovery_enabled===true,resource_discovery_json:String(saved.resource_discovery_json || '{}'),resource_discovery_limits_json:String(saved.resource_discovery_limits_json || '{}')});", "        Object.assign(state.settings,{resource_discovery_enabled:saved.resource_discovery_enabled===true,resource_discovery_json:String(saved.resource_discovery_json || '{}'),resource_discovery_limits_json:String(saved.resource_discovery_limits_json || '{}')});\r\n        state.settings.sitemap_source_processing_enabled = saved.sitemap_source_processing_enabled === true;\r\n        for (const field of sitemapSourceFields) state.settings[field.setting] = String(saved[field.setting] ?? field.initial);");
edit("    $('settings-form').elements.javascript_rendering.addEventListener('change', (event) => toggleRendererSettings(event.target.checked));", "    $('settings-form').elements.javascript_rendering.addEventListener('change', (event) => toggleRendererSettings(event.target.checked));\r\n    $('settings-form').elements.sitemap_source_processing_enabled.addEventListener('change', syncSitemapSourceProcessingSettings);");
edit("    $('advanced-config').append($('policy-settings-template').content.cloneNode(true));", "    $('advanced-config').append($('sitemap-source-processing-settings-template').content.cloneNode(true));\r\n    $('advanced-config').append($('policy-settings-template').content.cloneNode(true));");
edit("${state.settings.resource_discovery_enabled ? ' · Native Ressourcenregeln' : ''}", "${state.settings.resource_discovery_enabled ? ' · Native Ressourcenregeln' : ''}${state.settings.sitemap_source_processing_enabled ? ' · Eigene Sitemap-Grenzen' : ''}");
const helper=String.raw`  const sitemapSourceFields = [
    { setting:'sitemap_source_max_post_http_entity_bytes', native:'max_post_http_entity_bytes', initial:8388608, maximum:67108864, label:'Eingangsdaten der Sitemap' },
    { setting:'sitemap_source_max_decoded_xml_bytes', native:'max_decoded_xml_bytes', initial:8388608, maximum:67108864, label:'Entpackte XML-Daten' },
    { setting:'sitemap_source_max_xml_events', native:'max_xml_events', initial:500000, maximum:2000000, label:'XML-Ereignisse' }
  ];
  function readSitemapSourceProcessingSettings(form) {
    const result = { sitemap_source_processing_enabled:form.elements.sitemap_source_processing_enabled.checked };
    for (const field of sitemapSourceFields) result[field.setting] = form.elements.namedItem(field.setting).value;
    return result;
  }
  function syncSitemapSourceProcessingSettings() {
    const enabled = $('settings-form').elements.sitemap_source_processing_enabled.checked;
    const fields = $('sitemap-source-processing-fields');
    fields.disabled = !enabled;
    fields.hidden = !enabled;
  }
  function parseSitemapSourceProcessing(settings) {
    if (settings.sitemap_source_processing_enabled !== true) return {};
    const profile = {};
    for (const field of sitemapSourceFields) {
      const raw = settings[field.setting], value = Number(raw);
      if (!(typeof raw === 'number' || typeof raw === 'string' && /^[0-9]+$/.test(raw)) || !Number.isSafeInteger(value) || value < 1 || value > field.maximum) {
        throw new Error(field.label + ': Eine ganze Zahl von 1 bis ' + field.maximum + ' ist erforderlich.');
      }
      profile[field.native] = value;
    }
    return { sitemap_source_processing:profile };
  }

`;
edit('  function parseAdvanced(settings) {',helper.replace(/\n/g,'\r\n')+'  function parseAdvanced(settings) {');
const marker='  <template id="policy-settings-template">';insist(html.split(marker).length===2,'template marker changed');
const template=String.raw`  <template id="sitemap-source-processing-settings-template"><details id="sitemap-source-processing-settings" class="advanced-config"><summary>Verarbeitung von XML-Sitemap-Quellen</summary>
    <label class="checkbox-field"><input name="sitemap_source_processing_enabled" type="checkbox"><span>Eigene Verarbeitungsgrenzen für Sitemap-Quellen verwenden<small>Gilt für den nächsten Crawl. Ohne Aktivierung gelten die bisherigen Sitemap-Grenzen.</small></span></label>
    <fieldset id="sitemap-source-processing-fields" class="settings-grid" disabled hidden><legend>Arbeitsgrenzen für Sitemap-Dateien</legend>
      <label>Max. Eingangsdaten (Bytes)<input name="sitemap_source_max_post_http_entity_bytes" type="number" min="1" max="67108864" step="1" value="8388608" required><small>Nach HTTP-Dekomprimierung; 8.388.608 Bytes = 8 MiB, höchstens 64 MiB.</small></label>
      <label>Max. entpackte XML-Daten (Bytes)<input name="sitemap_source_max_decoded_xml_bytes" type="number" min="1" max="67108864" step="1" value="8388608" required><small>Nach zusätzlicher Gzip-Dateidekomprimierung, höchstens 64 MiB.</small></label>
      <label>Max. XML-Ereignisse<input name="sitemap_source_max_xml_events" type="number" min="1" max="2000000" step="1" value="500000" required><small>Begrenzt die Parserarbeit; höchstens 2.000.000 Ereignisse einschließlich Dateiende.</small></label>
    </fieldset>
    <p class="detail-empty">Die Antwortgröße für Seiten und die Speicherung von Sitemap-Mitgliedschaften haben eigene Grenzen. Erreichte Grenzen und unvollständige Messungen bleiben im Befund sichtbar.</p>
  </details></template>
`;
html=html.replace(marker,template.replace(/\n/g,'\r\n')+marker);
// Source generation only: complete before-image guards precede both writes.
fs.writeFileSync(appPath,app);fs.writeFileSync(indexPath,html);
process.stdout.write(JSON.stringify({result:'SOURCE_UI_PROFILE_AUTHORED_NOT_BROWSER_TESTED',before:{app:sha(appBefore),html:sha(indexBefore)},after:{app:sha(Buffer.from(app)),html:sha(Buffer.from(html))},compiler_browser_runs:0})+'\n');
