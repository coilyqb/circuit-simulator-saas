import { nanoid } from 'nanoid'
import { collectionNames, type CircuitDocument } from '../model/types'

export function documentIds(document: CircuitDocument): Set<string> {
  return new Set([document.id, ...collectionNames.flatMap(name => Object.keys(document[name]))])
}

/** Keep this allocator for the lifetime of an editing session, including undo and cancel. */
export class IdAllocator {
  private readonly used: Set<string>

  constructor(document: CircuitDocument) {
    this.used = documentIds(document)
  }

  claim(requested?: string): string {
    if (requested !== undefined) {
      if (!requested || ['__proto__', 'constructor', 'prototype'].includes(requested)
        || this.used.has(requested)) {
        throw new Error(`ID is invalid or already used: ${requested}`)
      }
      this.used.add(requested)
      return requested
    }
    let id: string
    do { id = nanoid(12) } while (this.used.has(id))
    this.used.add(id)
    return id
  }
}
