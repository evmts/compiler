#!/usr/bin/env node

const fs = require('node:fs/promises')
const { existsSync } = require('node:fs')
const path = require('node:path')

const rootDir = path.join(__dirname, '..')
const buildDir = path.join(rootDir, 'build')
const outputPath = path.join(buildDir, 'llms.md')
const testDir = path.join(rootDir, 'test')

const sections = [
	{
		tag: 'readme',
		path: path.join(rootDir, 'README.md'),
		format: 'markdown',
	},
	{
		tag: 'type-declarations',
		path: path.join(buildDir, 'index.d.ts'),
		format: 'typescript',
	},
]

function indent(level) {
	return '  '.repeat(level)
}

function indentBlock(content, level) {
	const prefix = indent(level)
	return content
		.trimEnd()
		.split('\n')
		.map((line) => `${prefix}${line}`)
		.join('\n')
}

async function collectSpecFiles(dir) {
	const entries = await fs.readdir(dir, { withFileTypes: true })
	const files = []

	for (const entry of entries) {
		const entryPath = path.join(dir, entry.name)

		if (entry.isDirectory()) {
			files.push(...(await collectSpecFiles(entryPath)))
		} else if (entry.isFile() && entry.name.endsWith('.spec.ts')) {
			files.push(entryPath)
		}
	}

	return files.sort((a, b) => a.localeCompare(b))
}

async function readFileOrThrow(filePath) {
	if (!existsSync(filePath)) {
		const relativePath = path.relative(rootDir, filePath)
		throw new Error(`Missing required file: ${relativePath}`)
	}

	return fs.readFile(filePath, 'utf8')
}

async function main() {
	await fs.mkdir(buildDir, { recursive: true })

	const lines = [
		'<llm-docs>',
		`${indent(1)}<intro>`,
		`${indent(2)}<title>LLM Quickstart Bundle</title>`,
		`${indent(2)}<description>This bundle provides core documentation, type definitions, and tests so an LLM can implement new features confidently.</description>`,
		`${indent(1)}</intro>`,
		'',
	]

	for (const section of sections) {
		const content = await readFileOrThrow(section.path)
		const attributes = ` format="${section.format}"`
		lines.push(`${indent(1)}<${section.tag}${attributes}>`)
		lines.push(indentBlock(content, 2))
		lines.push(`${indent(1)}</${section.tag}>`, '')
	}

	lines.push(
		`${indent(1)}<test-suite>`,
		`${indent(2)}<title>Test Suite</title>`,
		`${indent(2)}<description>The specs below define the expected behaviour across supported workflows. Treat them as the definitive acceptance criteria.</description>`,
	)

	const specFiles = await collectSpecFiles(testDir)

	if (specFiles.length === 0) {
		lines.push(`${indent(2)}<note>No test files found.</note>`)
	} else {
		for (const specFile of specFiles) {
			const content = await fs.readFile(specFile, 'utf8')
			const relativePath = path.relative(rootDir, specFile)
			lines.push(`${indent(2)}<spec-file path="${relativePath}" format="typescript">`)
			lines.push(indentBlock(content, 3))
			lines.push(`${indent(2)}</spec-file>`, '')
		}
	}

	lines.push(`${indent(1)}</test-suite>`, '</llm-docs>', '')

	const finalContent = lines.join('\n')
	await fs.writeFile(outputPath, finalContent, 'utf8')

	const relativeOutput = path.relative(rootDir, outputPath)
	console.log(`Wrote ${relativeOutput}`)
}

main().catch((error) => {
	console.error(error.message)
	process.exit(1)
})
