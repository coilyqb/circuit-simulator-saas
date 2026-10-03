import { applyPatches, enablePatches, freeze, produceWithPatches, type Draft, type Patch } from 'immer'
import { assertDocument, loadDocument, serializeDocument } from '../model/schema'
import type { CircuitDocument } from '../model/types'
import { getSymbol, type SymbolResolver } from '../symbols'
import { IdAllocator } from '../utils/ids'
import { remapSelection, type PasteOptions, type Selection } from '../utils/paste'
import { applyCommand, type Command } from './commands'

enablePatches()
interface UndoStep { patches: Patch[]; inverse: Patch[] }

export class DocumentHistory {
  private current: CircuitDocument
  private readonly ids: IdAllocator
  private readonly undoStack: UndoStep[] = []
  private readonly redoStack: UndoStep[] = []
  private transaction?: UndoStep
  private readonly resolve: SymbolResolver

  constructor(document: CircuitDocument, resolve: SymbolResolver = getSymbol) {
    this.resolve = resolve
    this.current = freeze(loadDocument(serializeDocument(document)), true)
    this.ids = new IdAllocator(this.current)
  }

  get document(): CircuitDocument { return this.current }
  get undoCount(): number { return this.undoStack.length }
  get redoCount(): number { return this.redoStack.length }
  get inTransaction(): boolean { return this.transaction !== undefined }

  private update(recipe: (draft: Draft<CircuitDocument>) => void) {
    const [next, patches, inverse] = produceWithPatches(this.current, recipe)
    assertDocument(next)
    if (!patches.length) return
    this.current = next
    if (this.transaction) {
      this.transaction.patches.push(...patches)
      this.transaction.inverse.unshift(...inverse)
    } else {
      this.undoStack.push({ patches, inverse })
      this.redoStack.length = 0
    }
  }

  execute(command: Command): string | undefined {
    // Detach input data so snapshots never freeze or retain caller-owned objects.
    const input = JSON.parse(JSON.stringify(command)) as Command
    let addedId: string | undefined
    this.update(draft => { addedId = applyCommand(draft, input, this.ids, this.resolve) })
    return addedId
  }

  beginTransaction() {
    if (this.transaction) throw new Error('A transaction is already active')
    this.transaction = { patches: [], inverse: [] }
  }

  commitTransaction() {
    if (!this.transaction) throw new Error('No active transaction')
    if (this.transaction.patches.length) {
      this.undoStack.push(this.transaction)
      this.redoStack.length = 0
    }
    this.transaction = undefined
  }

  cancelTransaction() {
    if (!this.transaction) throw new Error('No active transaction')
    this.current = freeze(applyPatches(this.current, this.transaction.inverse), true)
    this.transaction = undefined
  }

  undo(): boolean {
    if (this.transaction) throw new Error('Finish the transaction before undo')
    const step = this.undoStack.pop()
    if (!step) return false
    this.current = freeze(applyPatches(this.current, step.inverse), true)
    this.redoStack.push(step)
    return true
  }

  redo(): boolean {
    if (this.transaction) throw new Error('Finish the transaction before redo')
    const step = this.redoStack.pop()
    if (!step) return false
    this.current = freeze(applyPatches(this.current, step.patches), true)
    this.undoStack.push(step)
    return true
  }

  paste(source: CircuitDocument, selection: Selection, options: PasteOptions = {}): ReadonlyMap<string, string> {
    const result = remapSelection(source, this.current, selection, this.ids, options, this.resolve)
    this.update(draft => {
      for (const component of Object.values(result.fragment.components)) {
        draft.components[component.id] = component
      }
      Object.assign(draft.junctions, result.fragment.junctions)
      Object.assign(draft.wires, result.fragment.wires)
      Object.assign(draft.netLabels, result.fragment.netLabels)
      Object.assign(draft.ports, result.fragment.ports)
      Object.assign(draft.annotations, result.fragment.annotations)
    })
    return result.idMap
  }
}
