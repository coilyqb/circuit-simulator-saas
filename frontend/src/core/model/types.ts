export interface Point { x: number; y: number }
export type Rotation = 0 | 90 | 180 | 270
export interface Placement extends Point {
  rotation: Rotation
  mirror: 'none' | 'horizontal'
}
export type SymbolReference =
  | { kind: 'symbol'; symbolId: string; symbolVersion: number }
  | { kind: 'subcircuit'; definitionId: string; version: number }
export interface Component {
  id: string
  symbol: SymbolReference
  placement: Placement
  reference: string
  parameters: Record<string, string>
}
export type WireEndpoint =
  | { kind: 'pin'; componentId: string; pinId: string }
  | { kind: 'junction'; junctionId: string }
  | ({ kind: 'free' } & Point)
export interface Wire {
  id: string
  from: WireEndpoint
  to: WireEndpoint
  waypoints: Point[]
  routing: 'auto' | 'manual'
}
export interface Junction { id: string; position: Point }
export type LabelAttachment =
  | ({ kind: 'point' } & Point)
  | { kind: 'wire'; wireId: string }
  | { kind: 'pin'; componentId: string; pinId: string }
export interface NetLabel { id: string; name: string; attachment: LabelAttachment }
export interface Port {
  id: string
  name: string
  order: number
  direction?: 'in' | 'out' | 'inout'
  position: Point
}
export interface Annotation {
  id: string
  kind: 'text' | 'note'
  text: string
  position: Point
}
export interface CircuitDocument {
  schemaVersion: number
  kind: 'circuit' | 'subcircuit'
  id: string
  grid: number
  metadata: { name: string; description: string; category?: string; tags?: string[] }
  components: Record<string, Component>
  wires: Record<string, Wire>
  junctions: Record<string, Junction>
  netLabels: Record<string, NetLabel>
  ports: Record<string, Port>
  annotations: Record<string, Annotation>
}
export const collectionNames = [
  'components', 'wires', 'junctions', 'netLabels', 'ports', 'annotations',
] as const
export type CollectionName = typeof collectionNames[number]
