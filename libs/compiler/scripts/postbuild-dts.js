#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')

const rootDir = path.join(__dirname, '..')
const extendPath = path.join(rootDir, 'src', 'types', 'extend.ts')
const buildPath = path.join(rootDir, 'build', 'index.d.ts')

if (!fs.existsSync(extendPath) || !fs.existsSync(buildPath)) {
	process.exit(0)
}

const extendSource = fs.readFileSync(extendPath, 'utf8')
let buildSource = fs.readFileSync(buildPath, 'utf8')

const definitionPattern =
	/(?:^|\n)\s*(export\s+)?(type|interface|(?:declare\s+)?class)\s+([A-Za-z0-9_]+)[\s\S]*?(?=\n\s*(?:export\s+)?(?:type|interface|(?:declare\s+)?class)\s+[A-Za-z0-9_]+\b|\s*$)/g

const definitions = []
let match

while ((match = definitionPattern.exec(extendSource)) !== null) {
	const [block, , kind, name] = match
	const definition = block.trimStart().replace(/\s+$/, '')
	if (definition) {
		definitions.push({ kind, name, definition })
	}
}

if (definitions.length === 0) {
	process.exit(0)
}

for (const { kind, name, definition } of definitions) {
	const classPrefix =
		kind === 'interface'
			? '(?:declare\\s+)?(?:class|interface)'
			: kind.includes('class')
				? '(?:declare\\s+)?class'
				: kind
	const replacePattern = new RegExp(
		`(^|\\n)\\s*(?:export\\s+)?${classPrefix}\\s+${name}\\b[\\s\\S]*?(?=\\n\\s*(?:export\\s+)?(?:type|interface|(?:declare\\s+)?class)\\s+[A-Za-z0-9_]+\\b|\\s*$)`,
		'g',
	)

	let replaced = false
	buildSource = buildSource.replace(replacePattern, (_full, prefix) => {
		replaced = true
		return `${prefix}${definition}\n`
	})

	if (!replaced) {
		buildSource = `${buildSource.trimEnd()}\n\n${definition}\n`
	}
}

fs.writeFileSync(buildPath, buildSource)
