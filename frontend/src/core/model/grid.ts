/** Allow floating-point roundoff, but never a meaningful fraction of a grid unit. */
export function isGridCoordinate(coordinate: number, grid: number): boolean {
  if (!Number.isFinite(coordinate) || !Number.isFinite(grid) || grid <= 0) return false
  const units = coordinate / grid
  const nearest = Math.round(units)
  const tolerance = Math.min(1e-7, Number.EPSILON * Math.max(1, Math.abs(units)) * 4)
  return Number.isSafeInteger(nearest) && Math.abs(units - nearest) <= tolerance
}
