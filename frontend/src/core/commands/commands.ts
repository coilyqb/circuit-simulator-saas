import type { Draft } from 'immer'
import type {
  Annotation, CircuitDocument, Component, Junction, NetLabel, Point, Port, Rotation, Wire,
} from '../model/types'
import { getSymbol, type SymbolResolver } from '../symbols'
import type { IdAllocator } from '../utils/ids'

type New<T extends { id: string }> = Omit<T, 'id'> & { id?: string }
export type Command =
  | { type: 'addComponent'; component: Omit<New<Component>, 'reference' | 'parameters'> & {
    reference?: string; parameters?: Record<string, string>
  } }
  | { type: 'removeComponent'; id: string }
  | { type: 'moveComponent'; id: string; position: Point }
  | { type: 'rotateComponent'; id: string; rotation: Rotation }
  | { type: 'mirrorComponent'; id: string; mirror: Component['placement']['mirror'] }
  | { type: 'setParameter'; id: string; name: string; value: string }
  | { type: 'setReference'; id: string; reference: string }
  | { type: 'addWire'; wire: New<Wire> }
  | { type: 'removeWire'; id: string }
  | { type: 'setWireRouting'; id: string; waypoints: Point[]; routing: Wire['routing'] }
  | { type: 'addJunction'; junction: New<Junction> }
  | { type: 'removeJunction'; id: string }
  | { type: 'addNetLabel'; label: New<NetLabel> }
  | { type: 'removeNetLabel'; id: string }
  | { type: 'renameNetLabel'; id: string; name: string }
  | { type: 'addPort'; port: New<Port> }
  | { type: 'removePort'; id: string }
  | { type: 'renamePort'; id: string; name: string }
  | { type: 'addAnnotation'; annotation: New<Annotation> }
  | { type: 'updateAnnotation'; id: string; changes: Partial<Omit<Annotation, 'id'>> }
  | { type: 'removeAnnotation'; id: string }

export function assignReference(document: CircuitDocument, prefix: string): string {
  if (!/^[A-Za-z]+$/.test(prefix)) throw new Error('Invalid reference prefix')
  const used = new Set(Object.values(document.components).map(c => c.reference.toUpperCase()))
  let index = 1
  while (used.has(`${prefix}${index}`.toUpperCase())) index++
  return `${prefix}${index}`
}

function existing<T>(items: Record<string, T>, id: string): T {
  if (!Object.hasOwn(items, id)) throw new Error(`Unknown element ${id}`)
  return items[id]
}

function removeWire(document: Draft<CircuitDocument>, id: string) {
  existing(document.wires, id)
  for (const label of Object.values(document.netLabels)) {
    if (label.attachment.kind === 'wire' && label.attachment.wireId === id) delete document.netLabels[label.id]
  }
  delete document.wires[id]
}

/** Removal cascades delete attached wires and labels, never infer connectivity from geometry. */
export function applyCommand(document: Draft<CircuitDocument>, command: Command,
  ids: IdAllocator, resolve: SymbolResolver = getSymbol): string | undefined {
  switch (command.type) {
    case 'addComponent': {
      const input = command.component
      const definition = input.symbol.kind === 'symbol'
        ? resolve(input.symbol.symbolId, input.symbol.symbolVersion) : undefined
      const id = ids.claim(input.id)
      document.components[id] = {
        ...input, id, reference: input.reference ?? assignReference(document, definition?.spice.prefix ?? 'X'),
        parameters: { ...definition?.defaultParameters, ...input.parameters },
      }
      return id
    }
    case 'removeComponent':
      existing(document.components, command.id)
      for (const wire of Object.values(document.wires)) {
        if ([wire.from, wire.to].some(end => end.kind === 'pin' && end.componentId === command.id)) {
          removeWire(document, wire.id)
        }
      }
      for (const label of Object.values(document.netLabels)) {
        if (label.attachment.kind === 'pin' && label.attachment.componentId === command.id) {
          delete document.netLabels[label.id]
        }
      }
      delete document.components[command.id]
      break
    case 'moveComponent':
      Object.assign(existing(document.components, command.id).placement, command.position)
      break
    case 'rotateComponent':
      existing(document.components, command.id).placement.rotation = command.rotation
      break
    case 'mirrorComponent':
      existing(document.components, command.id).placement.mirror = command.mirror
      break
    case 'setParameter':
      if (['__proto__', 'constructor', 'prototype'].includes(command.name)) throw new Error('Invalid parameter name')
      existing(document.components, command.id).parameters[command.name] = command.value
      break
    case 'setReference':
      existing(document.components, command.id).reference = command.reference
      break
    case 'addWire': {
      const id = ids.claim(command.wire.id)
      document.wires[id] = { ...command.wire, id }
      return id
    }
    case 'removeWire':
      removeWire(document, command.id)
      break
    case 'setWireRouting':
      Object.assign(existing(document.wires, command.id), { waypoints: command.waypoints, routing: command.routing })
      break
    case 'addJunction': {
      const id = ids.claim(command.junction.id)
      document.junctions[id] = { ...command.junction, id }
      return id
    }
    case 'removeJunction':
      existing(document.junctions, command.id)
      for (const wire of Object.values(document.wires)) {
        if ([wire.from, wire.to].some(end => end.kind === 'junction' && end.junctionId === command.id)) {
          removeWire(document, wire.id)
        }
      }
      delete document.junctions[command.id]
      break
    case 'addNetLabel': {
      const id = ids.claim(command.label.id)
      document.netLabels[id] = { ...command.label, id }
      return id
    }
    case 'removeNetLabel':
      existing(document.netLabels, command.id)
      delete document.netLabels[command.id]
      break
    case 'renameNetLabel':
      existing(document.netLabels, command.id).name = command.name
      break
    case 'addPort': {
      const id = ids.claim(command.port.id)
      document.ports[id] = { ...command.port, id }
      return id
    }
    case 'removePort':
      existing(document.ports, command.id)
      delete document.ports[command.id]
      break
    case 'renamePort':
      existing(document.ports, command.id).name = command.name
      break
    case 'addAnnotation': {
      const id = ids.claim(command.annotation.id)
      document.annotations[id] = { ...command.annotation, id }
      return id
    }
    case 'updateAnnotation':
      Object.assign(existing(document.annotations, command.id), command.changes)
      break
    case 'removeAnnotation':
      existing(document.annotations, command.id)
      delete document.annotations[command.id]
      break
  }
}
