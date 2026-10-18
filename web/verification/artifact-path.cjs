'use strict';
const fs = require('node:fs');
const path = require('node:path');

// Runtime evidence must never overwrite tracked fixtures or source witnesses.
const directory = path.resolve(process.env.SPIDER_TEST_ARTIFACT_DIR ||
  path.join(__dirname, '../../artifacts/ui-verification'));
fs.mkdirSync(directory, { recursive: true });

module.exports = function artifactPath(filename) {
  if (path.basename(filename) !== filename || !filename || filename === '.' || filename === '..') {
    throw new Error('Test artifact name must be a single filename');
  }
  return path.join(directory, filename);
};
