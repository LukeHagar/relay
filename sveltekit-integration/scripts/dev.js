#!/usr/bin/env node

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..');

// Start SvelteKit dev server
const svelteProcess = spawn('npm', ['run', 'dev'], {
	cwd: projectRoot,
	stdio: 'inherit',
	shell: true
});

// Start WebSocket server
const wsProcess = spawn('node', ['scripts/websocket-server.js'], {
	cwd: projectRoot,
	stdio: 'inherit',
	shell: true
});

// Handle process cleanup
process.on('SIGINT', () => {
	console.log('\nShutting down servers...');
	svelteProcess.kill('SIGINT');
	wsProcess.kill('SIGINT');
	process.exit(0);
});

process.on('SIGTERM', () => {
	svelteProcess.kill('SIGTERM');
	wsProcess.kill('SIGTERM');
	process.exit(0);
});

console.log('Starting SvelteKit development environment...');
console.log('SvelteKit: http://localhost:5173');
console.log('WebSocket: ws://localhost:4001');
console.log('Press Ctrl+C to stop all servers');