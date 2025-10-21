#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const rootDir = path.join(__dirname, '..')
const extendPath = path.join(rootDir, 'src', 'types', 'extensions.ts')
const buildPath = path.join(rootDir, 'build', 'index.d.ts')

if (!fs.existsSync(extendPath) || !fs.existsSync(buildPath)) {
	process.exit(0)
}

const extendSource = fs.readFileSync(extendPath, 'utf8')
let buildSource = fs.readFileSync(buildPath, 'utf8')

const extendDefinitions = collectExtendDefinitions(extendSource)

if (extendDefinitions.length === 0) {
	process.exit(0)
}

const buildDefinitions = collectBuildDefinitions(buildSource)
const replacements = []
const additions = []

for (const definition of extendDefinitions) {
	const buildDefinition = takeBuildDefinition(buildDefinitions.map, definition.name, definition.kind)

	if (buildDefinition) {
		const comment = definition.comment ?? buildDefinition.comment
		const block = renderBlock(
			buildDefinition.prefix,
			buildDefinition.indent,
			comment,
			definition.declaration,
			buildDefinition.memberComments ?? null,
		)

		replacements.push({
			start: buildDefinition.replaceStart,
			end: buildDefinition.replaceEnd,
			text: block,
		})
	} else {
		additions.push(definition)
	}
}

if (replacements.length > 0) {
	replacements.sort((a, b) => b.start - a.start)

	for (const replacement of replacements) {
		buildSource = buildSource.slice(0, replacement.start) + replacement.text + buildSource.slice(replacement.end)
	}
}

if (additions.length > 0) {
	const additionText = additions
		.map((definition) => renderBlock('', '', definition.comment, definition.declaration, null))
		.join('\n\n')

	const trailingWhitespace = buildSource.match(/\s*$/)?.[0] ?? ''
	const trimmed = buildSource.slice(0, buildSource.length - trailingWhitespace.length)
	const separator = trimmed ? '\n\n' : ''

	buildSource = `${trimmed}${separator}${additionText}\n`
} else if (!buildSource.endsWith('\n')) {
	buildSource = `${buildSource}\n`
}

fs.writeFileSync(buildPath, buildSource)

function collectExtendDefinitions(sourceText) {
	const sourceFile = ts.createSourceFile('extensions.ts', sourceText, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)

	const definitions = []

	const visit = (node) => {
		const kind = getDeclarationKind(node)

		if (kind && node.name) {
			const name = node.name.text
			const commentInfo = extractCommentInfo(sourceText, node)
			const leadingEnd = commentInfo ? commentInfo.start : node.getStart(sourceFile)
			const leadingText = sourceText.slice(node.getFullStart(), leadingEnd)
			const { indent: commentIndent } = splitLeading(leadingText)
			const declarationIndent = getIndentForPosition(sourceText, node.getStart(sourceFile))

			const declarationText = sourceText.slice(node.getStart(sourceFile), node.getEnd())

			definitions.push({
				name,
				kind,
				comment: commentInfo ? { text: commentInfo.text, indent: commentIndent } : null,
				declaration: {
					text: declarationText,
					indent: declarationIndent,
					members: collectMemberDescriptors(node, sourceFile, sourceText),
				},
			})
		}

		ts.forEachChild(node, visit)
	}

	visit(sourceFile)

	return definitions
}

function collectBuildDefinitions(sourceText) {
	const sourceFile = ts.createSourceFile('build.d.ts', sourceText, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)

	const map = new Map()

	const visit = (node) => {
		const kind = getDeclarationKind(node)

		if (kind && node.name) {
			const name = node.name.text
			const commentInfo = extractCommentInfo(sourceText, node)
			const leadingEnd = commentInfo ? commentInfo.start : node.getStart(sourceFile)
			const leadingText = sourceText.slice(node.getFullStart(), leadingEnd)
			const { prefix, indent } = splitLeading(leadingText)
			const declarationIndent = getIndentForPosition(sourceText, node.getStart(sourceFile))

			const comment = commentInfo?.text ? { text: commentInfo.text, indent } : null

			const entry = {
				name,
				kind,
				prefix,
				indent: declarationIndent,
				comment,
				memberComments: collectMemberComments(node, sourceFile, sourceText),
				replaceStart: node.getFullStart(),
				replaceEnd: node.getEnd(),
			}

			const list = map.get(name)
			if (list) {
				list.push(entry)
			} else {
				map.set(name, [entry])
			}
		}

		ts.forEachChild(node, visit)
	}

	visit(sourceFile)

	return { map }
}

function getDeclarationKind(node) {
	if (ts.isInterfaceDeclaration(node)) {
		return 'interface'
	}

	if (ts.isClassDeclaration(node)) {
		return 'class'
	}

	if (ts.isTypeAliasDeclaration(node)) {
		return 'type'
	}

	return null
}

function takeBuildDefinition(map, name, kind) {
	const list = map.get(name)

	if (!list || list.length === 0) {
		return null
	}

	let index = list.findIndex((entry) => entry.kind === kind)
	if (index === -1) {
		index = 0
	}

	const [entry] = list.splice(index, 1)

	if (list.length === 0) {
		map.delete(name)
	}

	return entry
}

function extractCommentInfo(sourceText, node) {
	const docs = ts.getJSDocCommentsAndTags(node)

	if (!docs || docs.length === 0) {
		return null
	}

	const start = docs[0].getFullStart()
	const end = docs[docs.length - 1].getEnd()
	const text = sourceText.slice(start, end)

	return { start, end, text }
}

function splitLeading(text) {
	if (!text) {
		return { prefix: '', indent: '' }
	}

	const lastNewline = text.lastIndexOf('\n')

	if (lastNewline === -1) {
		return { prefix: '', indent: text }
	}

	return {
		prefix: text.slice(0, lastNewline + 1),
		indent: text.slice(lastNewline + 1),
	}
}

function getIndentForPosition(sourceText, position) {
	if (position === 0) {
		return ''
	}

	let index = position - 1

	while (index >= 0 && sourceText[index] !== '\n') {
		index -= 1
	}

	return sourceText.slice(index + 1, position)
}

function renderBlock(prefix, targetIndent, comment, declaration, memberComments) {
	const commentPart = comment ? `${formatComment(comment.text, targetIndent, comment.indent)}\n` : ''
	const declarationSource =
		memberComments && declaration.members && declaration.members.length
			? injectMemberComments(declaration.text, declaration.members, memberComments)
			: declaration.text
	const declarationPart = formatDeclaration(declarationSource, targetIndent, declaration.indent)

	return `${prefix}${commentPart}${declarationPart}`
}

function formatComment(text, targetIndent, existingIndent) {
	const lines = text.split('\n').map((line) => line.replace(/\s+$/, ''))
	const normalised = lines.map((line, index) => {
		if (index === 0) {
			return line.trimStart()
		}

		const withoutIndent = removeIndentPrefix(line, existingIndent).replace(/^\s*/, '')

		if (!withoutIndent) {
			return ''
		}

		if (withoutIndent.startsWith('*')) {
			return ` ${withoutIndent}`
		}

		return withoutIndent
	})

	if (!targetIndent) {
		return normalised.join('\n').trimEnd()
	}

	return normalised
		.map((line, index) => {
			if (!line) {
				return ''
			}

			if (index === 0) {
				return `${targetIndent}${line}`
			}

			return line.startsWith(' ') ? `${targetIndent}${line.slice(1)}` : `${targetIndent}${line}`
		})
		.join('\n')
		.trimEnd()
}

function formatDeclaration(text, targetIndent, existingIndent) {
	const lines = text.split('\n').map((line) => line.replace(/\s+$/, ''))

	return lines
		.map((line) => {
			const withoutIndent = removeIndentPrefix(line, existingIndent)

			if (!targetIndent) {
				return withoutIndent
			}

			return withoutIndent ? `${targetIndent}${withoutIndent}` : ''
		})
		.join('\n')
		.trimEnd()
}

function collectMemberDescriptors(node, sourceFile, sourceText) {
	if (!('members' in node) || !node.members) {
		return []
	}

	const baseStart = node.getStart(sourceFile)
	const descriptors = []

	for (const member of node.members) {
		const key = getMemberKey(member, sourceFile)

		if (!key) {
			continue
		}

		const memberStart = member.getStart(sourceFile)
		const indent = getIndentForPosition(sourceText, memberStart)
		const relativeStart = memberStart - indent.length - baseStart

		descriptors.push({
			key,
			relativeStart: relativeStart >= 0 ? relativeStart : 0,
			indent,
		})
	}

	return descriptors.sort((a, b) => a.relativeStart - b.relativeStart)
}

function collectMemberComments(node, sourceFile, sourceText) {
	if (!('members' in node) || !node.members) {
		return null
	}

	const comments = new Map()

	for (const member of node.members) {
		const key = getMemberKey(member, sourceFile)

		if (!key) {
			continue
		}

		const commentInfo = extractCommentInfo(sourceText, member)

		if (!commentInfo) {
			continue
		}

		const leadingText = sourceText.slice(member.getFullStart(), commentInfo.start)
		const { indent } = splitLeading(leadingText)
		const entry = { text: commentInfo.text, indent }

		const existing = comments.get(key)

		if (existing) {
			existing.push(entry)
		} else {
			comments.set(key, [entry])
		}
	}

	return comments.size > 0 ? comments : null
}

function getMemberKey(member, sourceFile) {
	if (!member.name) {
		return null
	}

	if (ts.isIdentifier(member.name) || ts.isStringLiteral(member.name) || ts.isNumericLiteral(member.name)) {
		return member.name.text
	}

	return member.name.getText(sourceFile)
}

function injectMemberComments(text, members, memberComments) {
	let result = text
	let offset = 0

	for (const member of members) {
		const bucket = memberComments?.get(member.key)

		if (!bucket || bucket.length === 0) {
			continue
		}

		const info = bucket.shift()

		const commentText = formatComment(info.text, member.indent, info.indent)
		const insertion = `${commentText}\n`
		const insertPosition = member.relativeStart + offset

		result = result.slice(0, insertPosition) + insertion + result.slice(insertPosition)
		offset += insertion.length
	}

	return result
}

function removeIndentPrefix(line, indent) {
	if (!indent || !line) {
		return line
	}

	let index = 0

	while (index < indent.length && index < line.length && indent[index] === line[index]) {
		index += 1
	}

	return line.slice(index)
}
