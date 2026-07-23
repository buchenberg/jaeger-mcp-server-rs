#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const BIN_EXT = process.platform === 'win32' ? '.exe' : '';
const BIN_PATH = path.join(__dirname, 'bin', `jaeger-mcp-server${BIN_EXT}`);

if (!fs.existsSync(BIN_PATH)) {
  console.error(
    `[jaeger-mcp-server] Binary not found at ${BIN_PATH}.\n` +
    `Run "npm install" to download it.`
  );
  process.exit(1);
}

const result = spawnSync(BIN_PATH, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env,
});

process.exit(result.status ?? 1);
