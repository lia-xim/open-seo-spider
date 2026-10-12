import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, stat } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, 'docs/parity');
const executable = process.env.OPEN_SEO_REFERENCE_CLI ?? 'C:/Program Files (x86)/Screaming Frog SEO Spider/ScreamingFrogSEOSpiderCli.exe';
const configuration = resolve(process.env.USERPROFILE ?? '', '.ScreamingFrogSEOSpider/spider.config');
async function hash(path) { try { return createHash('sha256').update(await readFile(path)).digest('hex'); } catch (error) { if (error.code === 'ENOENT') return null; throw error; } }
async function help(option) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(executable, ['--help', ...(option ? [option] : [])], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const stdout = [], stderr = [];
    const timeout = setTimeout(() => { child.kill(); reject(new Error('Reference help timed out after 30s')); }, 30000);
    child.stdout.on('data', buffer => stdout.push(buffer));
    child.stderr.on('data', buffer => stderr.push(buffer));
    child.on('error', error => { clearTimeout(timeout); reject(error); });
    child.on('close', code => {
      clearTimeout(timeout);
      if (code !== 0) return reject(new Error(`Reference help exit ${code}`));
      resolvePromise({ arguments: ['--help', ...(option ? [option] : [])], exit_code: code,
        stdout: new TextDecoder('windows-1252').decode(Buffer.concat(stdout)), stderr_bytes: Buffer.concat(stderr).length });
    });
  });
}
await mkdir(output, { recursive: true });
const before = await hash(configuration);
const options = ['', 'export-tabs', 'bulk-export', 'save-report', 'export-custom-summary'];
const runs = [];
for (const option of options) runs.push({ option: option || 'general', ...await help(option) });
const exports = runs.filter(run => run.option !== 'general').map(run => ({ category: run.option,
  arguments: run.stdout.split(/\r?\n/).map(line => line.trim()).filter(line => line && !line.startsWith('Die Option') && !line.startsWith('The ') && !line.startsWith('Option ')) }));
const entries = exports.find(item => item.category === 'export-tabs').arguments.filter(line => line.includes(':')).map(argument => {
  const separator = argument.indexOf(':');
  const tab = argument.slice(0, separator), filter = argument.slice(separator + 1);
  const slot = /^(?:(?:OpenAI|Ollama|Gemini|Anthropic) Prompt|Extractor|Search|Filter|Snippet) \d+$/.test(filter);
  return { argument, tab, filter, kind: filter === 'All' ? 'aggregate' : slot ? 'configured_slot' : 'named_filter',
    instance_template: slot ? filter.replace(/\d+$/, '{n}') : null,
    parity_status: 'NOT_PROVEN', semantics_status: 'REQUIRES_ACCEPTANCE_SPECIFICATION' };
});
const result = { schema_version: 1, observed_at: new Date().toISOString(), product: 'Screaming Frog SEO Spider',
  version_basis: 'Installed artifact manifest retained in task work/screaming-frog-package-observations.json; marketed v24.3.',
  executable: { basename: 'ScreamingFrogSEOSpiderCli.exe', size_bytes: (await stat(executable)).size, sha256: await hash(executable) },
  evidence_kind: 'Installed original CLI help output; process executed with help only, no crawl or target website request.',
  profile_configuration_sha256: { before, after: await hash(configuration) },
  licensing: { file_existence_checked: true, file_contents_read: false, help_available: true, crawl_export_availability: 'NOT_PROVEN' },
  counts: { exported_tabs: new Set(entries.map(entry => entry.tab)).size, total_export_tab_arguments: entries.length,
    named_filters: entries.filter(entry => entry.kind === 'named_filter').length,
    configured_slots: entries.filter(entry => entry.kind === 'configured_slot').length,
    aggregates: entries.filter(entry => entry.kind === 'aggregate').length },
  export_tab_entries: entries, other_export_arguments: exports.filter(item => item.category !== 'export-tabs'),
  general_options: [...runs[0].stdout.matchAll(/^\s+(--[a-z0-9-]+)/gm)].map(match => match[1]),
  source_urls: ['https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#command-line-interface'],
  limitations: ['Help lists advertised argument names, not runtime rule correctness or licensing entitlement.', 'Configured slots are not separate audit algorithms.', 'No translations, vendor method bodies, assets or implementation source are retained in this project.'] };
await writeFile(resolve(output, 'reference-cli-inventory.json'), JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify({ output: 'docs/parity/reference-cli-inventory.json', counts: result.counts,
  configuration_unchanged: before === result.profile_configuration_sha256.after }));
