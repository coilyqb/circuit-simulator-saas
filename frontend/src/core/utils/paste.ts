import { assignReference } from '../commands/commands'
import { assertDocument, createDocument, loadDocument, serializeDocument } from '../model/schema'
import { collectionNames, type CircuitDocument, type CollectionName, type Point, type WireEndpoint } from '../model/types'
import { componentPinPosition, getSymbol, type SymbolResolver } from '../symbols'
import { IdAllocator } from './ids'

export type Selection = Partial<Record<CollectionName, readonly string[]>>
export interface PasteOptions {
  offset?: Point
  /** Required only for external subcircuit pins; positions are in source document coordinates. */
  resolveExternalPin?: (componentId: string, pinId: string) => Point
}

/** External wire ends become free; labels attached to unselected pins/wires are dropped. */
export function remapSelection(source: CircuitDocument, target: CircuitDocument, selection: Selection,
  ids: IdAllocator, options: PasteOptions = {}, resolve: SymbolResolver = getSymbol) {
  assertDocument(source)
  assertDocument(target)
  if (source.grid !== target.grid) throw new Error('Paste requires matching grid units')
  const offset = options.offset ?? { x: 0, y: 0 }
  const translate = (point: Point): Point => ({ x: point.x + offset.x, y: point.y + offset.y })
  const copy = loadDocument(serializeDocument(source))
  const fragment = createDocument({ id: target.id, kind: target.kind, grid: target.grid })
  const idMap = new Map<string, string>()
  const selected = (name: CollectionName, id: string) => (selection[name] ?? []).includes(id)
  for (const name of collectionNames) {
    for (const oldId of new Set(selection[name] ?? [])) {
      if (!Object.hasOwn(source[name], oldId)) throw new Error(`Unknown selected element ${oldId}`)
      if (name === 'netLabels') {
        const attachment = source.netLabels[oldId].attachment
        if (attachment.kind === 'pin' && !selected('components', attachment.componentId)) continue
        if (attachment.kind === 'wire' && !selected('wires', attachment.wireId)) continue
      }
      idMap.set(oldId, ids.claim())
    }
  }
  const references = loadDocument(serializeDocument(target))
  for (const oldId of selection.components ?? []) {
    if (fragment.components[idMap.get(oldId)!]) continue
    const component = copy.components[oldId]
    const id = idMap.get(oldId)!
    const prefix = component.symbol.kind === 'symbol'
      ? resolve(component.symbol.symbolId, component.symbol.symbolVersion).spice.prefix : 'X'
    component.id = id
    component.placement = { ...component.placement, ...translate(component.placement) }
    component.reference = assignReference(references, prefix)
    references.components[id] = component
    fragment.components[id] = component
  }
  for (const oldId of new Set(selection.junctions ?? [])) {
    const junction = copy.junctions[oldId]
    junction.id = idMap.get(oldId)!
    junction.position = translate(junction.position)
    fragment.junctions[junction.id] = junction
  }
  const endpoint = (end: WireEndpoint): WireEndpoint => {
    if (end.kind === 'free') return { kind: 'free', ...translate(end) }
    const oldId = end.kind === 'pin' ? end.componentId : end.junctionId
    const id = idMap.get(oldId)
    if (id) return end.kind === 'pin' ? { ...end, componentId: id } : { ...end, junctionId: id }
    const position = end.kind === 'junction' ? source.junctions[end.junctionId].position
      : source.components[end.componentId].symbol.kind === 'subcircuit'
        ? options.resolveExternalPin?.(end.componentId, end.pinId)
        : componentPinPosition(source.components[end.componentId], end.pinId, source.grid, resolve)
    if (!position) throw new Error('External subcircuit pin position resolver required')
    return { kind: 'free', ...translate(position) }
  }
  for (const oldId of new Set(selection.wires ?? [])) {
    const wire = copy.wires[oldId]
    wire.id = idMap.get(oldId)!
    wire.from = endpoint(wire.from)
    wire.to = endpoint(wire.to)
    wire.waypoints = wire.waypoints.map(translate)
    fragment.wires[wire.id] = wire
  }
  for (const oldId of new Set(selection.netLabels ?? [])) {
    const id = idMap.get(oldId)
    if (!id) continue
    const label = copy.netLabels[oldId]
    label.id = id
    const attachment = label.attachment
    label.attachment = attachment.kind === 'point' ? { ...attachment, ...translate(attachment) }
      : attachment.kind === 'pin' ? { ...attachment, componentId: idMap.get(attachment.componentId)! }
        : { ...attachment, wireId: idMap.get(attachment.wireId)! }
    fragment.netLabels[id] = label
  }
  const orders = new Set(Object.values(target.ports).map(port => port.order))
  for (const oldId of new Set(selection.ports ?? [])) {
    const port = copy.ports[oldId]
    port.id = idMap.get(oldId)!
    while (orders.has(port.order)) port.order++
    orders.add(port.order)
    port.position = translate(port.position)
    fragment.ports[port.id] = port
  }
  for (const oldId of new Set(selection.annotations ?? [])) {
    const annotation = copy.annotations[oldId]
    annotation.id = idMap.get(oldId)!
    annotation.position = translate(annotation.position)
    fragment.annotations[annotation.id] = annotation
  }
  assertDocument(fragment)
  return { fragment, idMap: idMap as ReadonlyMap<string, string> }
}

export function cloneDocument(source: CircuitDocument, resolve: SymbolResolver = getSymbol): CircuitDocument {
  const ids = new IdAllocator(source)
  const target = createDocument({ id: ids.claim(), kind: source.kind, grid: source.grid })
  const selection = Object.fromEntries(collectionNames.map(name => [name, Object.keys(source[name])])) as Selection
  const { fragment } = remapSelection(source, target, selection, ids, {}, resolve)
  const clone = { ...fragment, id: target.id, metadata: JSON.parse(JSON.stringify(source.metadata)) }
  assertDocument(clone)
  return clone
}
