// Exact default Source28 MCP contract. Optional file tools are deliberately excluded.
import assert from 'node:assert/strict';

export const EXPECTED_DEFAULT_MCP_NAMES = Object.freeze([
  "spider_analysis",
  "spider_analyze_content",
  "spider_analyze_crawl_content",
  "spider_api_enrichment",
  "spider_broken_sources",
  "spider_cancel",
  "spider_capabilities",
  "spider_compare",
  "spider_compare_saved",
  "spider_content",
  "spider_content_document",
  "spider_crawl_quotas",
  "spider_generate_sitemaps",
  "spider_inspect_css",
  "spider_issues",
  "spider_list_crawls",
  "spider_ngrams",
  "spider_pages",
  "spider_pause",
  "spider_policy",
  "spider_preview_policy",
  "spider_project",
  "spider_recrawl",
  "spider_resource_transports",
  "spider_resources",
  "spider_resume",
  "spider_segments",
  "spider_sitemap_findings",
  "spider_sitemaps",
  "spider_spelling",
  "spider_start_crawl",
  "spider_status"
]);

export function assertDefaultMcpInventory(tools) {
  assert(Array.isArray(tools), 'Default MCP tools must be an array');
  assert.equal(tools.length, 32, 'Default MCP tool count changed');
  const actual = tools.map(tool => {
    assert(tool && typeof tool === 'object' && !Array.isArray(tool), 'Invalid MCP tool');
    assert.equal(typeof tool.name, 'string', 'MCP tool name must be a string');
    return tool.name;
  });
  assert.equal(new Set(actual).size, 32, 'Duplicate MCP tool names');
  assert.deepEqual([...actual].sort(), [...EXPECTED_DEFAULT_MCP_NAMES], 'Default MCP tool names changed');
  return actual;
}
