import { nanoid } from 'nanoid'
import { collectionNames, type CircuitDocument } from './types'

export const SCHEMA_VERSION = 1
export type Migration = (document: Record<string, unknown>) => Record<string, unknown>
export type Migrations = Readonly<Record<number, Migration>>
// Each step is keyed by its input version and must advance exactly one version.
export const migrations: Migrations = {
  0: document => ({ ...document, schemaVersion: 1, annotations: document.annotations ?? {} }),
}

function requireValue(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(`Invalid circuit document: ${message}`)
}
function record(value: unknown): asserts value is Record<string, unknown> {
  requireValue(value !== null && typeof value === 'object' && !Array.isArray(value), 'expected object')
}
function text(value: unknown): asserts value is string {
  requireValue(typeof value === 'string', 'expected string')
}
function id(value: unknown): asserts value is string {
  text(value)
  requireValue(value.length > 0 && !['__proto__', 'constructor', 'prototype'].includes(value), 'invalid ID')
}
function integer(value: unknown, minimum = 0): asserts value is number {
  requireValue(typeof value === 'number' && Number.isSafeInteger(value) && value >= minimum, 'invalid integer')
}
function point(value: unknown, grid: number) {
  record(value)
  for (const coordinate of [value.x, value.y]) {
    requireValue(typeof coordinate === 'number' && Number.isFinite(coordinate)
      && Number.isSafeInteger(coordinate / grid), 'position must be on grid')
  }
}

/** Validates storage structure, not derived electrical validation or subcircuit pin definitions. */
export function assertDocument(value: unknown): asserts value is CircuitDocument {
  record(value)
  requireValue(value.schemaVersion === SCHEMA_VERSION, 'unsupported schema version')
  requireValue(value.kind === 'circuit' || value.kind === 'subcircuit', 'invalid kind')
  id(value.id)
  requireValue(typeof value.grid === 'number' && Number.isFinite(value.grid) && value.grid > 0, 'invalid grid')
  const grid = value.grid as number
  record(value.metadata)
  text(value.metadata.name)
  text(value.metadata.description)
  if (value.metadata.category !== undefined) text(value.metadata.category)
  if (value.metadata.tags !== undefined) {
    requireValue(Array.isArray(value.metadata.tags), 'invalid tags')
    value.metadata.tags.forEach(text)
  }
  const used = new Set([value.id])
  for (const name of collectionNames) {
    const items = value[name]
    record(items)
    for (const [key, item] of Object.entries(items)) {
      id(key)
      record(item)
      requireValue(item.id === key && !used.has(key), 'duplicate or mismatched ID')
      used.add(key)
    }
  }
  const document = value as unknown as CircuitDocument
  const references = new Set<string>()
  for (const component of Object.values(document.components)) {
    record(component.symbol)
    if (component.symbol.kind === 'symbol') {
      id(component.symbol.symbolId)
      integer(component.symbol.symbolVersion, 1)
    } else {
      requireValue(component.symbol.kind === 'subcircuit', 'invalid symbol reference')
      id(component.symbol.definitionId)
      integer(component.symbol.version, 1)
    }
    point(component.placement, grid)
    requireValue([0, 90, 180, 270].includes(component.placement.rotation), 'invalid rotation')
    requireValue(['none', 'horizontal'].includes(component.placement.mirror), 'invalid mirror')
    text(component.reference)
    requireValue(component.reference.trim().length > 0
      && !references.has(component.reference.toUpperCase()), 'reference designators must be unique')
    references.add(component.reference.toUpperCase())
    record(component.parameters)
    Object.values(component.parameters).forEach(text)
    requireValue(!('pins' in component), 'pins are derived, not stored')
  }
  const pin = (attachment: { componentId: string; pinId: string }) => {
    id(attachment.componentId)
    id(attachment.pinId)
    requireValue(Object.hasOwn(document.components, attachment.componentId), 'missing component')
  }
  const endpoint = (attachment: unknown) => {
    record(attachment)
    if (attachment.kind === 'pin') pin(attachment as unknown as { componentId: string; pinId: string })
    else if (attachment.kind === 'junction') {
      id(attachment.junctionId)
      requireValue(Object.hasOwn(document.junctions, attachment.junctionId), 'missing junction')
    } else {
      requireValue(attachment.kind === 'free', 'invalid endpoint')
      point(attachment, grid)
    }
  }
  for (const wire of Object.values(document.wires)) {
    endpoint(wire.from)
    endpoint(wire.to)
    requireValue(Array.isArray(wire.waypoints), 'invalid waypoints')
    wire.waypoints.forEach(p => point(p, grid))
    requireValue(['auto', 'manual'].includes(wire.routing), 'invalid routing')
  }
  for (const junction of Object.values(document.junctions)) point(junction.position, grid)
  for (const label of Object.values(document.netLabels)) {
    text(label.name)
    record(label.attachment)
    if (label.attachment.kind === 'pin') pin(label.attachment)
    else if (label.attachment.kind === 'wire') {
      id(label.attachment.wireId)
      requireValue(Object.hasOwn(document.wires, label.attachment.wireId), 'missing wire')
    } else {
      requireValue((label.attachment as { kind: string }).kind === 'point', 'invalid label attachment')
      point(label.attachment, grid)
    }
  }
  requireValue(document.kind === 'subcircuit' || Object.keys(document.ports).length === 0,
    'ports require a subcircuit')
  const orders = new Set<number>()
  for (const port of Object.values(document.ports)) {
    text(port.name)
    integer(port.order)
    requireValue(!orders.has(port.order), 'duplicate port order')
    orders.add(port.order)
    requireValue(port.direction === undefined || ['in', 'out', 'inout'].includes(port.direction), 'invalid direction')
    point(port.position, grid)
  }
  for (const annotation of Object.values(document.annotations)) {
    requireValue(['text', 'note'].includes(annotation.kind), 'invalid annotation kind')
    text(annotation.text)
    point(annotation.position, grid)
  }
}

export function createDocument(options: {
  id?: string; kind?: CircuitDocument['kind']; grid?: number; name?: string
} = {}): CircuitDocument {
  const document: CircuitDocument = {
    schemaVersion: SCHEMA_VERSION, kind: options.kind ?? 'circuit',
    id: options.id ?? nanoid(12), grid: options.grid ?? 1,
    metadata: { name: options.name ?? 'Untitled', description: '' },
    components: {}, wires: {}, junctions: {}, netLabels: {}, ports: {}, annotations: {},
  }
  assertDocument(document)
  return document
}

export function loadDocument(json: string, steps: Migrations = migrations): CircuitDocument {
  let value: unknown = JSON.parse(json)
  record(value)
  integer(value.schemaVersion)
  if (value.schemaVersion > SCHEMA_VERSION) {
    throw new Error(`Unsupported newer schema version ${value.schemaVersion}; current is ${SCHEMA_VERSION}`)
  }
  while ((value.schemaVersion as number) < SCHEMA_VERSION) {
    const version = value.schemaVersion as number
    const step = steps[version]
    if (!step) throw new Error(`No migration for schema version ${version}`)
    value = step(value)
    record(value)
    requireValue(value.schemaVersion === version + 1, 'migration must advance one schema version')
  }
  assertDocument(value)
  return value
}

export function serializeDocument(document: CircuitDocument): string {
  assertDocument(document)
  return JSON.stringify(document)
}
