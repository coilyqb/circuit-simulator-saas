import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, it } from 'vitest'

const root = fileURLToPath(new URL('../src/core/', import.meta.url))
function sourceFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? sourceFiles(path)
      : path.endsWith('.ts') && !path.endsWith('.test.ts') ? [path] : []
  })
}

it('keeps production core imports inside core or the two pure runtime dependencies', () => {
  for (const path of sourceFiles(root)) {
    const source = readFileSync(path, 'utf8')
    expect(source, path).not.toMatch(/\b(?:require|import)\s*\(/)
    for (const match of source.matchAll(/\b(?:from|import)\s*['"]([^'"]+)['"]/g)) {
      const specifier = match[1]
      if (specifier.startsWith('.')) {
        expect(resolve(dirname(path), specifier).startsWith(root), path).toBe(true)
      } else {
        expect(['immer', 'nanoid'], path).toContain(specifier)
      }
    }
  }
})
