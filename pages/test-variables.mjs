#!/usr/bin/env node
/**
 * Regression test for CSS variables
 * Verifies variables.scss matches SCSS files that use them
 */

import { readFileSync, readdirSync, statSync } from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const srcPath = path.join(__dirname, 'src');
const variablesScssPath = path.join(__dirname, 'src/styles/variables.scss');

let variablesScss;
try {
  variablesScss = readFileSync(variablesScssPath, 'utf-8');
} catch (err) {
  console.error(`Failed to read file: ${variablesScssPath}`);
  process.exit(1);
}

const indexVars = new Map();
const files = readdirSync(srcPath, { recursive: true });
const varUsageRegex = /var\(--([\w-]+)\)/g;
let match;

for (const file of files) {
    varUsageRegex.lastIndex = 0;
    const filePath = path.join(srcPath, file);
    if (statSync(filePath).isFile() && filePath.endsWith('.scss')) {
        const content = readFileSync(filePath, 'utf-8');
        while ((match = varUsageRegex.exec(content)) !== null) {
            const name = '--' + match[1];
            indexVars.set(name, 'used');
        }
    }
}

const scssVars = new Map();
const scssRegex = /--([\w-]+):\s*#\{(\$[\w-]+)\};/g;

while ((match = scssRegex.exec(variablesScss)) !== null) {
    const name = '--' + match[1];
    const sassVar = match[2];
    scssVars.set(name, sassVar);
}

console.log('Comparing CSS variables between SCSS files and variables.scss\n');

const matches = new Set([...indexVars.keys()].filter(k => scssVars.has(k)));
const passed = matches.size;
const failed = new Set([...indexVars.keys(), ...scssVars.keys()]).size - passed;

indexVars.forEach((value, name) => {
    if (scssVars.has(name)) {
        console.log(`✓ ${name}: ${value}`);
    } else {
        console.log(`✗ ${name}: MISSING in variables.scss`);
    }
});

scssVars.forEach((value, name) => {
    if (!indexVars.has(name)) {
        console.log(`✗ ${name}: MISSING in SCSS files`);
    }
});

console.log(`\n${passed} variables found, ${failed} missing\n`);

if (failed > 0) {
    console.error('FAIL: Some CSS variables are missing from variables.scss or SCSS files');
    process.exit(1);
}

console.log('PASS: All CSS variables are present in both SCSS files and variables.scss');
process.exit(0);
