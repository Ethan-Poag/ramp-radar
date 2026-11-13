import type { DistUnit, Status } from './api';

const NM: Record<DistUnit, number> = { nm: 1, mi: 0.868976, km: 0.539957 };
export const toUnit = (nm: number, u: DistUnit) => nm / NM[u];
export const fromUnit = (v: number, u: DistUnit) => v * NM[u];

export function fmtDist(nm: number | null | undefined, u: DistUnit): string {
  if (nm == null) return '—';
  const v = toUnit(nm, u);
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${u}`;
}
export function fmtAlt(ft: number | null | undefined): string {
  if (ft == null) return '—';
  return ft <= 0 ? 'GND' : `${ft.toLocaleString()} ft`;
}
export function fmtEta(min: number | null | undefined): string {
  if (min == null) return '—';
  if (min < 1) return '<1 min';
  if (min < 90) return `${Math.round(min)} min`;
  return `${Math.floor(min / 60)} h ${Math.round(min % 60)} m`;
}
export function ago(iso: string | null | undefined, now = Date.now()): string {
  if (!iso) return 'never';
  const s = Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000));
  if (s < 5) return 'just now';
  if (s < 90) return `${s} s ago`;
  if (s < 5400) return `${Math.round(s / 60)} min ago`;
  if (s < 172800) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86400)} d ago`;
}
export const STATUS_LABEL: Record<Status, string> = {
  not_seen: 'Not seen',
  inbound: 'Inbound',
  within: 'Within radius',
  on_ground: 'On ground',
  outbound: 'Outbound',
  disabled: 'Disabled',
};
export const minToTime = (m: number) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
export const timeToMin = (t: string) => {
  const [h, m] = t.split(':').map(Number);
  return (h || 0) * 60 + (m || 0);
};

/** Circle polygon as GeoJSON ring coordinates [lon,lat]. */
export function circle(lat: number, lon: number, rNm: number, steps = 96): [number, number][] {
  const R = 3440.065;
  const d = rNm / R;
  const p1 = (lat * Math.PI) / 180;
  const l1 = (lon * Math.PI) / 180;
  const pts: [number, number][] = [];
  for (let i = 0; i <= steps; i++) {
    const b = (i / steps) * 2 * Math.PI;
    const p2 = Math.asin(Math.sin(p1) * Math.cos(d) + Math.cos(p1) * Math.sin(d) * Math.cos(b));
    const l2 = l1 + Math.atan2(Math.sin(b) * Math.sin(d) * Math.cos(p1), Math.cos(d) - Math.sin(p1) * Math.sin(p2));
    pts.push([(l2 * 180) / Math.PI, (p2 * 180) / Math.PI]);
  }
  return pts;
}
