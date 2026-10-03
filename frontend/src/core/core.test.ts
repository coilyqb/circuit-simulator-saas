import { describe, expect, it } from 'vitest'
import {
  builtInSymbols, cloneDocument, collectionNames, createDocument, DocumentHistory, getSymbol,
  loadDocument, pinPositions, serializeDocument,
  type CircuitDocument, type Command, type Placement, type Rotation, type WireEndpoint,
} from './index'

const placement: Placement = { x: 0, y: 0, rotation: 0, mirror: 'none' }
function addComponent(history: DocumentHistory, symbolId = 'resistor', id?: string): string {
  return history.execute({
    type: 'addComponent',
    component: { id, symbol: { kind: 'symbol', symbolId, symbolVersion: 1 }, placement },
  })!
}
const pin = (componentId: string, pinId = '1'): WireEndpoint => ({ kind: 'pin', componentId, pinId })
function addWire(history: DocumentHistory, from: WireEndpoint, to: WireEndpoint): string {
  return history.execute({ type: 'addWire', wire: { from, to, waypoints: [], routing: 'auto' } })!
}
function rcCircuit() {
  const empty = createDocument({ grid: 10 })
  const history = new DocumentHistory(empty)
  const voltage = addComponent(history, 'voltageSource')
  const resistor = addComponent(history)
  const capacitor = addComponent(history, 'capacitor')
  const ground = addComponent(history, 'ground')
  history.execute({ type: 'moveComponent', id: resistor, position: { x: 40, y: 0 } })
  history.execute({ type: 'moveComponent', id: capacitor, position: { x: 80, y: 0 } })
  history.execute({ type: 'moveComponent', id: ground, position: { x: 80, y: 40 } })
  history.execute({ type: 'setParameter', id: resistor, name: 'value', value: '10k' })
  history.execute({ type: 'setParameter', id: capacitor, name: 'value', value: '100n' })
  const junction = history.execute({ type: 'addJunction', junction: { position: { x: 60, y: 0 } } })!
  const inputWire = addWire(history, pin(voltage, '+'), pin(resistor))
  const outputWire = addWire(history, pin(resistor, '2'), { kind: 'junction', junctionId: junction })
  addWire(history, { kind: 'junction', junctionId: junction }, pin(capacitor))
  addWire(history, pin(capacitor, '2'), pin(ground, '0'))
  addWire(history, pin(voltage, '-'), pin(ground, '0'))
  const label = history.execute({
    type: 'addNetLabel', label: { name: 'OUT', attachment: { kind: 'wire', wireId: outputWire } },
  })!
  return { empty, history, voltage, resistor, capacitor, ground, junction, inputWire, outputWire, label }
}

describe('document commands and history', () => {
  it('builds an RC circuit, undoes to empty, and redoes losslessly', () => {
    const { empty, history, resistor, capacitor } = rcCircuit()
    const completed = history.document
    expect(Object.keys(completed.components)).toHaveLength(4)
    expect(Object.keys(completed.wires)).toHaveLength(5)
    expect(Object.keys(completed.junctions)).toHaveLength(1)
    expect(completed.components[resistor].parameters).toEqual({ value: '10k' })
    expect(completed.components[capacitor].parameters).toEqual({ value: '100n' })
    const steps = history.undoCount
    for (let i = 0; i < steps; i++) expect(history.undo()).toBe(true)
    expect(history.document).toEqual(empty)
    expect(history.undo()).toBe(false)
    for (let i = 0; i < steps; i++) expect(history.redo()).toBe(true)
    expect(history.document).toEqual(completed)
    expect(history.redo()).toBe(false)
  })

  it('groups 100 moves in one undo step and cancels another gesture', () => {
    const history = new DocumentHistory(createDocument())
    const id = addComponent(history)
    const original = history.document
    history.beginTransaction()
    for (let x = 1; x <= 100; x++) {
      history.execute({ type: 'moveComponent', id, position: { x, y: 0 } })
    }
    expect(history.undoCount).toBe(1)
    history.commitTransaction()
    expect(history.undoCount).toBe(2)
    const moved = history.document
    history.undo()
    expect(history.document).toEqual(original)
    history.redo()
    expect(history.document).toEqual(moved)
    history.beginTransaction()
    for (let x = 101; x <= 200; x++) history.execute({ type: 'moveComponent', id, position: { x, y: 0 } })
    history.cancelTransaction()
    expect(history.document).toEqual(moved)
    expect(history.undoCount).toBe(2)
  })

  it('guards transaction lifecycle, preserves redo on cancel, and clears it on new edits', () => {
    const history = new DocumentHistory(createDocument())
    const id = addComponent(history)
    history.execute({ type: 'setParameter', id, name: 'value', value: '2k' })
    history.undo()
    history.beginTransaction()
    expect(() => history.beginTransaction()).toThrow('already active')
    expect(() => history.undo()).toThrow('transaction')
    expect(() => history.redo()).toThrow('transaction')
    history.execute({ type: 'setParameter', id, name: 'value', value: '3k' })
    history.cancelTransaction()
    expect(history.redoCount).toBe(1)
    history.beginTransaction()
    history.commitTransaction()
    expect(history.redoCount).toBe(1)
    history.execute({ type: 'setParameter', id, name: 'value', value: '4k' })
    expect(history.redoCount).toBe(0)
    expect(() => history.cancelTransaction()).toThrow('No active')
    expect(() => history.commitTransaction()).toThrow('No active')
  })

  it('auto-assigns unique references, rejects collisions, and allows user edits', () => {
    const history = new DocumentHistory(createDocument())
    const first = addComponent(history)
    const second = addComponent(history)
    const capacitor = addComponent(history, 'capacitor')
    expect(history.document.components[first].reference).toBe('R1')
    expect(history.document.components[second].reference).toBe('R2')
    expect(history.document.components[capacitor].reference).toBe('C1')
    history.execute({ type: 'setReference', id: first, reference: 'r3' })
    expect(() => history.execute({ type: 'setReference', id: second, reference: 'R3' })).toThrow('unique')
    expect(history.document.components[second].reference).toBe('R2')
    const third = addComponent(history)
    const fourth = addComponent(history)
    expect(history.document.components[third].reference).toBe('R1')
    expect(history.document.components[fourth].reference).toBe('R4')
  })

  it('deletes attached wires and labels with a component and restores them together', () => {
    const { history, resistor, inputWire, outputWire, label } = rcCircuit()
    const pinLabel = history.execute({
      type: 'addNetLabel', label: { name: 'IN', attachment: { kind: 'pin', componentId: resistor, pinId: '1' } },
    })!
    const original = history.document
    history.execute({ type: 'removeComponent', id: resistor })
    expect(history.document.components[resistor]).toBeUndefined()
    expect(history.document.wires[inputWire]).toBeUndefined()
    expect(history.document.wires[outputWire]).toBeUndefined()
    expect(history.document.netLabels[label]).toBeUndefined()
    expect(history.document.netLabels[pinLabel]).toBeUndefined()
    expect(Object.keys(history.document.wires)).toHaveLength(3)
    history.undo()
    expect(history.document).toEqual(original)
    history.redo()
    expect(history.document.components[resistor]).toBeUndefined()
  })

  it('covers rotations, mirrors, routing, labels, ports, and annotations with undo/redo', () => {
    const history = new DocumentHistory(createDocument({ kind: 'subcircuit' }))
    const empty = history.document
    const component = addComponent(history)
    const junction = history.execute({ type: 'addJunction', junction: { position: { x: 4, y: 0 } } })!
    const wire = addWire(history, pin(component), { kind: 'junction', junctionId: junction })
    const label = history.execute({ type: 'addNetLabel', label: { name: 'A', attachment: { kind: 'wire', wireId: wire } } })!
    const port = history.execute({ type: 'addPort', port: { name: 'IN', order: 0, direction: 'in', position: { x: -2, y: 0 } } })!
    const note = history.execute({ type: 'addAnnotation', annotation: { kind: 'note', text: 'original', position: { x: 0, y: 2 } } })!
    const commands: Command[] = [
      { type: 'rotateComponent', id: component, rotation: 90 },
      { type: 'mirrorComponent', id: component, mirror: 'horizontal' },
      { type: 'setWireRouting', id: wire, waypoints: [{ x: 2, y: 2 }], routing: 'manual' },
      { type: 'renameNetLabel', id: label, name: 'B' },
      { type: 'renamePort', id: port, name: 'INPUT' },
      { type: 'updateAnnotation', id: note, changes: { text: 'updated', position: { x: 2, y: 2 } } },
    ]
    commands.forEach(command => history.execute(command))
    expect(history.document.ports[port]).toMatchObject({ id: port, name: 'INPUT' })
    expect(history.document.wires[wire]).toMatchObject({ routing: 'manual', waypoints: [{ x: 2, y: 2 }] })
    history.execute({ type: 'removeNetLabel', id: label })
    history.execute({ type: 'removeWire', id: wire })
    history.execute({ type: 'removeJunction', id: junction })
    history.execute({ type: 'removePort', id: port })
    history.execute({ type: 'removeAnnotation', id: note })
    const final = history.document
    while (history.undo()) { /* Undo all commands. */ }
    expect(history.document).toEqual(empty)
    while (history.redo()) { /* Replay all commands. */ }
    expect(history.document).toEqual(final)
  })

  it('cascades junction deletion through wires and wire labels', () => {
    const { history, junction, outputWire, label } = rcCircuit()
    const original = history.document
    history.execute({ type: 'removeJunction', id: junction })
    expect(history.document.wires[outputWire]).toBeUndefined()
    expect(history.document.netLabels[label]).toBeUndefined()
    expect(Object.keys(history.document.wires)).toHaveLength(3)
    history.undo()
    expect(history.document).toEqual(original)
  })

  it('never reuses IDs after deletion, undo, or transaction cancellation', () => {
    const history = new DocumentHistory(createDocument({ id: 'document' }))
    addComponent(history, 'resistor', 'first')
    history.execute({ type: 'removeComponent', id: 'first' })
    expect(() => addComponent(history, 'capacitor', 'first')).toThrow('already used')
    addComponent(history, 'resistor', 'second')
    history.undo()
    expect(() => addComponent(history, 'resistor', 'second')).toThrow('already used')
    history.beginTransaction()
    addComponent(history, 'resistor', 'third')
    history.cancelTransaction()
    expect(() => addComponent(history, 'resistor', 'third')).toThrow('already used')
    expect(() => addComponent(history, 'resistor', 'document')).toThrow('already used')
    expect(() => addComponent(history, 'resistor', '__proto__')).toThrow('invalid')
  })

  it('rejects invalid commands atomically and keeps snapshots independent of callers', () => {
    const document = createDocument()
    const history = new DocumentHistory(document)
    const id = addComponent(history)
    document.metadata.name = 'mutated'
    expect(history.document.metadata.name).toBe('Untitled')
    expect(Object.isFrozen(history.document.components[id])).toBe(true)
    expect(Object.isFrozen(placement)).toBe(false)
    const original = history.document
    expect(() => history.execute({ type: 'moveComponent', id, position: { x: 0.5, y: 0 } })).toThrow('grid')
    expect(() => addWire(history, pin('missing'), pin(id))).toThrow('missing component')
    expect(() => history.execute({ type: 'setParameter', id, name: '__proto__', value: 'bad' })).toThrow('parameter')
    expect(history.document).toBe(original)
    expect(history.undoCount).toBe(1)
    expect(() => getSymbol('resistor', 2)).toThrow('Unknown symbol')
    expect(() => history.execute({ type: 'removeComponent', id: 'missing' })).toThrow('Unknown element')
    const another = new DocumentHistory(createDocument())
    expect(another.undoCount).toBe(0)
  })
})

describe('paste and clone', () => {
  it('remaps every internal reference and makes external connections free in one step', () => {
    const { history, resistor, capacitor, junction, inputWire, outputWire, label } = rcCircuit()
    const pinLabel = history.execute({
      type: 'addNetLabel', label: { name: 'PIN', attachment: { kind: 'pin', componentId: capacitor, pinId: '1' } },
    })!
    const pointLabel = history.execute({
      type: 'addNetLabel', label: { name: 'POINT', attachment: { kind: 'point', x: 60, y: 0 } },
    })!
    const externalLabel = history.execute({
      type: 'addNetLabel', label: { name: 'EXTERNAL', attachment: { kind: 'pin', componentId: Object.keys(history.document.components)[0], pinId: '+' } },
    })!
    const note = history.execute({
      type: 'addAnnotation', annotation: { kind: 'text', text: 'RC', position: { x: 0, y: 0 } },
    })!
    const original = history.document
    const steps = history.undoCount
    const selectedWires = Object.values(original.wires)
      .filter(w => w.id === inputWire || w.id === outputWire || w.from.kind === 'junction').map(w => w.id)
    const map = history.paste(original, {
      components: [resistor, capacitor], junctions: [junction], wires: selectedWires,
      netLabels: [label, pinLabel, pointLabel, externalLabel], annotations: [note],
    }, { offset: { x: 100, y: 100 } })
    expect(history.undoCount).toBe(steps + 1)
    const oldIds = new Set([original.id, ...collectionNames.flatMap(name => Object.keys(original[name]))])
    for (const id of map.values()) expect(oldIds.has(id)).toBe(false)
    expect(new Set(map.values()).size).toBe(map.size)
    expect(map.has(externalLabel)).toBe(false)
    const pasted = history.document
    expect(pasted.wires[map.get(inputWire)!].from).toEqual({ kind: 'free', x: 80, y: 100 })
    expect(pasted.wires[map.get(inputWire)!].to).toEqual(pin(map.get(resistor)!))
    expect(pasted.wires[map.get(outputWire)!].to).toEqual({ kind: 'junction', junctionId: map.get(junction) })
    expect(pasted.netLabels[map.get(label)!].attachment).toEqual({ kind: 'wire', wireId: map.get(outputWire) })
    expect(pasted.netLabels[map.get(pinLabel)!].attachment).toEqual(pin(map.get(capacitor)!))
    expect(pasted.netLabels[map.get(pointLabel)!].attachment).toEqual({ kind: 'point', x: 160, y: 100 })
    expect(pasted.annotations[map.get(note)!].position).toEqual({ x: 100, y: 100 })
    expect(pasted.components[map.get(resistor)!].reference).toBe('R2')
    expect(original.components[resistor].placement.x).toBe(40)
    history.undo()
    expect(history.document).toEqual(original)
    history.redo()
    expect(history.document).toEqual(pasted)
  })

  it('clones a complete document including ports and pinned subcircuit references', () => {
    const history = new DocumentHistory(createDocument({ kind: 'subcircuit' }))
    const component = history.execute({ type: 'addComponent', component: {
      symbol: { kind: 'subcircuit', definitionId: 'definition', version: 7 }, placement,
    } })!
    const port = history.execute({ type: 'addPort', port: { name: 'IN', order: 0, position: { x: -2, y: 0 } } })!
    addWire(history, pin(component, port), { kind: 'free', x: 4, y: 0 })
    const source = history.document
    const clone = cloneDocument(source)
    const oldIds = new Set([source.id, ...collectionNames.flatMap(name => Object.keys(source[name]))])
    for (const id of [clone.id, ...collectionNames.flatMap(name => Object.keys(clone[name]))]) {
      expect(oldIds.has(id)).toBe(false)
    }
    const newComponent = Object.values(clone.components)[0]
    expect(newComponent.symbol).toEqual({ kind: 'subcircuit', definitionId: 'definition', version: 7 })
    // Instance pin IDs belong to the pinned external definition, not this document.
    expect(Object.values(clone.wires)[0].from).toEqual(pin(newComponent.id, port))
    expect(clone.metadata).toEqual(source.metadata)
    expect(loadDocument(serializeDocument(clone))).toEqual(clone)
  })

  it('drops labels on unselected wires and frees unselected junction endpoints', () => {
    const { history, outputWire, label, junction } = rcCircuit()
    const source = history.document
    const map = history.paste(source, { wires: [outputWire], netLabels: [label] })
    expect(history.document.wires[map.get(outputWire)!].to).toEqual({ kind: 'free', ...source.junctions[junction].position })
    const labelOnly = history.paste(source, { netLabels: [label] })
    expect(labelOnly.size).toBe(0)
  })

  it('handles duplicate selections and rejects incompatible grids and invalid offsets', () => {
    const { history, resistor, junction } = rcCircuit()
    const original = history.document
    const map = history.paste(original, { components: [resistor, resistor], junctions: [junction, junction] },
      { offset: { x: 10, y: 0 } })
    expect(map.size).toBe(2)
    expect(history.document.junctions[map.get(junction)!].position).toEqual({ x: 70, y: 0 })
    expect(() => history.paste(createDocument(), {})).toThrow('matching grid')
    expect(() => history.paste(original, { components: [resistor] }, { offset: { x: 1, y: 0 } })).toThrow('grid')
    expect(() => history.paste(original, { components: ['missing'] })).toThrow('Unknown selected')
  })

  it('assigns non-conflicting port orders when pasting into a subcircuit', () => {
    const history = new DocumentHistory(createDocument({ kind: 'subcircuit' }))
    const port = history.execute({ type: 'addPort', port: { name: 'IN', order: 0, position: { x: 0, y: 0 } } })!
    const map = history.paste(history.document, { ports: [port] })
    expect(history.document.ports[map.get(port)!]).toMatchObject({ name: 'IN', order: 1 })
  })
})

describe('symbols and storage', () => {
  it('keeps all built-in pins on grid for every rotation and mirror', () => {
    for (const definition of Object.values(builtInSymbols)) {
      expect(Object.isFrozen(definition)).toBe(true)
      expect(Object.isFrozen(definition.pins)).toBe(true)
      expect(definition.spice.pinOrder).toEqual(definition.pins.map(pin => pin.id))
      for (const pin of definition.pins) {
        expect(Number.isInteger(pin.position.x)).toBe(true)
        expect(Number.isInteger(pin.position.y)).toBe(true)
      }
      for (const grid of [1, 10, 0.5]) {
        for (const rotation of [0, 90, 180, 270] as Rotation[]) {
          for (const mirror of ['none', 'horizontal'] as const) {
            const positions = pinPositions(definition, { x: 4 * grid, y: -2 * grid, rotation, mirror }, grid)
            for (const point of Object.values(positions)) {
              expect(Number.isInteger(point.x / grid)).toBe(true)
              expect(Number.isInteger(point.y / grid)).toBe(true)
            }
          }
        }
      }
    }
    expect(builtInSymbols.ground.spice.globalNet).toBe('0')
    expect(Object.values(builtInSymbols).filter(s => s.spice.globalNet)).toHaveLength(1)
    expect(pinPositions(builtInSymbols.bjt, { ...placement, rotation: 90, mirror: 'horizontal' }, 10).base)
      .toEqual({ x: 0, y: 20 })
  })

  it('upgrades v0 through a migration and rejects missing, newer, or broken migrations', () => {
    const current = createDocument()
    const { annotations: _, ...rest } = current
    const old = { ...rest, schemaVersion: 0 }
    expect(loadDocument(JSON.stringify(old))).toEqual(current)
    let calls = 0
    expect(loadDocument(JSON.stringify(old), {
      0: input => { calls++; return { ...input, schemaVersion: 1, annotations: {} } },
    })).toEqual(current)
    expect(calls).toBe(1)
    expect(() => loadDocument(JSON.stringify(old), {})).toThrow('No migration for schema version 0')
    expect(() => loadDocument(JSON.stringify({ ...current, schemaVersion: 99 }))).toThrow('Unsupported newer schema version 99')
    expect(() => loadDocument(JSON.stringify(old), { 0: input => input })).toThrow('advance one')
    expect(() => loadDocument(JSON.stringify({ ...current, schemaVersion: 'unknown' }))).toThrow('integer')
  })

  it('round-trips JSON without storing derived data or losing raw strings', () => {
    const { history } = rcCircuit()
    const document = history.document
    const json = serializeDocument(document)
    expect(loadDocument(json)).toEqual(document)
    expect(serializeDocument(loadDocument(json))).toBe(json)
    for (const derived of ['nets', 'pinPositions', 'validationResults', 'netlist', 'version']) {
      expect(document).not.toHaveProperty(derived)
    }
    for (const component of Object.values(document.components)) expect(component).not.toHaveProperty('pins')
  })

  it('rejects corrupt IDs, references, raw parameters, and grid coordinates on load', () => {
    const { history, resistor } = rcCircuit()
    const corrupt = (change: (document: CircuitDocument) => void) => {
      const document = loadDocument(serializeDocument(history.document))
      change(document)
      return () => loadDocument(JSON.stringify(document))
    }
    expect(corrupt(d => { d.components[resistor].id = 'other' })).toThrow('mismatched')
    expect(corrupt(d => { d.junctions[resistor] = { id: resistor, position: { x: 0, y: 0 } } })).toThrow('duplicate')
    expect(corrupt(d => { d.components[resistor].parameters.value = 10 as unknown as string })).toThrow('string')
    expect(corrupt(d => { d.components[resistor].placement.x = 1 })).toThrow('grid')
    expect(corrupt(d => { Object.values(d.wires)[0].from = pin('missing') })).toThrow('missing component')
    expect(() => createDocument({ grid: 0 })).toThrow('grid')
  })
})
