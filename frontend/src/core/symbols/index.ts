import { freeze } from 'immer'
import type { Component, Placement, Point } from '../model/types'

export interface SymbolPin { readonly id: string; readonly name: string; readonly position: Readonly<Point> }
export type SymbolGeometry =
  | { readonly kind: 'polyline'; readonly points: readonly Readonly<Point>[] }
  | { readonly kind: 'circle'; readonly center: Readonly<Point>; readonly radius: number }
  | { readonly kind: 'arc'; readonly center: Readonly<Point>; readonly radius: number;
    readonly startAngle: number; readonly endAngle: number }
export interface SymbolDefinition {
  readonly id: string
  readonly version: number
  readonly name: string
  readonly pins: readonly SymbolPin[]
  readonly geometry: readonly SymbolGeometry[]
  readonly defaultParameters: Readonly<Record<string, string>>
  readonly spice: {
    readonly prefix: string
    readonly pinOrder: readonly string[]
    readonly globalNet?: '0'
  }
}

function symbol(id: string, prefix: string, pins: SymbolPin[],
  defaults: Record<string, string>, geometry: SymbolGeometry[], ground = false): SymbolDefinition {
  return freeze({
    id, version: 1, name: id, pins,
    geometry,
    defaultParameters: defaults,
    spice: { prefix, pinOrder: pins.map(pin => pin.id), ...(ground ? { globalNet: '0' as const } : {}) },
  }, true)
}
const pin = (id: string, x: number, y: number): SymbolPin => ({ id, name: id, position: { x, y } })
const polyline = (...points: Point[]): SymbolGeometry => ({ kind: 'polyline', points })
const twoPins = [pin('1', -2, 0), pin('2', 2, 0)]
const line = [polyline({ x: -2, y: 0 }, { x: 2, y: 0 })]
const sourceOutline: SymbolGeometry[] = [
  polyline({ x: -2, y: 0 }, { x: -1, y: 0 }),
  { kind: 'circle', center: { x: 0, y: 0 }, radius: 1 },
  polyline({ x: 1, y: 0 }, { x: 2, y: 0 }),
]
export const builtInSymbols: Readonly<Record<string, SymbolDefinition>> = freeze({
  resistor: symbol('resistor', 'R', twoPins, { value: '1k' },
    [polyline({ x: -2, y: 0 }, { x: -1, y: 1 }, { x: 0, y: -1 }, { x: 1, y: 1 }, { x: 2, y: 0 })]),
  capacitor: symbol('capacitor', 'C', twoPins, { value: '1u' }, [
    polyline({ x: -2, y: 0 }, { x: -0.5, y: 0 }),
    polyline({ x: -0.5, y: -1 }, { x: -0.5, y: 1 }),
    polyline({ x: 0.5, y: -1 }, { x: 0.5, y: 1 }),
    polyline({ x: 0.5, y: 0 }, { x: 2, y: 0 }),
  ]),
  inductor: symbol('inductor', 'L', twoPins, { value: '1m' }, [
    polyline({ x: -2, y: 0 }, { x: -1.5, y: 0 }),
    ...[-1, 0, 1].map(x => ({
      kind: 'arc' as const, center: { x, y: 0 }, radius: 0.5, startAngle: 180, endAngle: 0,
    })),
    polyline({ x: 1.5, y: 0 }, { x: 2, y: 0 }),
  ]),
  voltageSource: symbol('voltageSource', 'V', [pin('+', -2, 0), pin('-', 2, 0)], { value: 'DC 1' }, [
    ...sourceOutline,
    polyline({ x: -0.75, y: 0 }, { x: -0.25, y: 0 }),
    polyline({ x: -0.5, y: -0.25 }, { x: -0.5, y: 0.25 }),
    polyline({ x: 0.25, y: 0 }, { x: 0.75, y: 0 }),
  ]),
  currentSource: symbol('currentSource', 'I', [pin('+', -2, 0), pin('-', 2, 0)], { value: 'DC 1m' }, [
    ...sourceOutline,
    polyline({ x: -0.5, y: 0 }, { x: 0.5, y: 0 }),
    polyline({ x: 0.25, y: -0.25 }, { x: 0.5, y: 0 }, { x: 0.25, y: 0.25 }),
  ]),
  ground: symbol('ground', 'G', [pin('0', 0, 0)], {},
    [polyline({ x: 0, y: 0 }, { x: 0, y: 1 }),
      polyline({ x: -1, y: 1 }, { x: 1, y: 1 }),
      polyline({ x: -0.5, y: 1.5 }, { x: 0.5, y: 1.5 }),
      polyline({ x: -0.25, y: 2 }, { x: 0.25, y: 2 })], true),
  diode: symbol('diode', 'D', [pin('anode', -2, 0), pin('cathode', 2, 0)], { model: '' }, line),
  bjt: symbol('bjt', 'Q', [pin('collector', 0, -2), pin('base', -2, 0), pin('emitter', 0, 2)], { model: '' }, line),
  mosfet: symbol('mosfet', 'M', [pin('drain', 0, -2), pin('gate', -2, 0), pin('source', 0, 2), pin('bulk', 2, 0)],
    { model: '', W: '1u', L: '1u' }, line),
}, true)

export type SymbolResolver = (id: string, version: number) => SymbolDefinition
export const getSymbol: SymbolResolver = (id, version) => {
  const definition = Object.hasOwn(builtInSymbols, id) ? builtInSymbols[id] : undefined
  if (!definition || definition.version !== version) throw new Error(`Unknown symbol ${id}@${version}`)
  return definition
}

/** Local coordinates use grid units; mirror local X first, then rotate counter-clockwise. */
export function transformPoint(point: Readonly<Point>, placement: Placement, grid: number): Point {
  const x = placement.mirror === 'horizontal' ? -point.x : point.x
  const y = point.y
  const rotated = placement.rotation === 0 ? { x, y }
    : placement.rotation === 90 ? { x: -y, y: x }
      : placement.rotation === 180 ? { x: -x, y: -y } : { x: y, y: -x }
  return { x: placement.x + rotated.x * grid, y: placement.y + rotated.y * grid }
}

export function pinPositions(definition: SymbolDefinition, placement: Placement, grid: number): Record<string, Point> {
  return Object.fromEntries(definition.pins.map(pin => [pin.id, transformPoint(pin.position, placement, grid)]))
}

export function componentPinPosition(component: Component, pinId: string, grid: number,
  resolve: SymbolResolver = getSymbol): Point {
  if (component.symbol.kind !== 'symbol') throw new Error('Subcircuit pin positions require a definition resolver')
  const definition = resolve(component.symbol.symbolId, component.symbol.symbolVersion)
  const pin = definition.pins.find(pin => pin.id === pinId)
  if (!pin) throw new Error(`Unknown pin ${pinId}`)
  return transformPoint(pin.position, component.placement, grid)
}
