export type DistUnit = 'nm' | 'mi' | 'km';
export type ChannelKind = 'desktop' | 'sound' | 'ntfy' | 'email' | 'webhook' | 'twilio';
export type Target = { kind: 'reg' | 'hex' | 'callsign'; value: string };
export type Status = 'not_seen' | 'inbound' | 'within' | 'on_ground' | 'outbound' | 'disabled';

export interface Tracker {
  id: string;
  label: string;
  target: Target;
  airport: string;
  radius: { value: number; unit: DistUnit };
  alert_enter: boolean;
  alert_exit: boolean;
  alert_landing: boolean;
  alert_takeoff: boolean;
  ceiling_ft: number | null;
  quiet_hours: { start_min: number; end_min: number } | null;
  channels: ChannelKind[];
  color: string;
  enabled: boolean;
}

export interface TrackerState {
  tracker_id: string;
  status: Status;
  inside: boolean;
  distance_nm: number | null;
  eta_min: number | null;
  alt_ft: number | null;
  gs_kt: number | null;
  track: number | null;
  lat: number | null;
  lon: number | null;
  hex: string | null;
  reg: string | null;
  callsign: string | null;
  type_code: string | null;
  last_seen: string | null;
  provider: string | null;
  trail: { lat: number; lon: number; t: number }[];
}

export interface Airport {
  ident: string;
  iata: string | null;
  name: string;
  kind: string;
  city: string | null;
  country: string;
  lat: number;
  lon: number;
  elevation_ft: number | null;
}

export interface Aircraft {
  hex: string;
  reg: string | null;
  callsign: string | null;
  type_code: string | null;
  lat: number | null;
  lon: number | null;
  alt_ft: number | null;
  on_ground: boolean;
  gs_kt: number | null;
  track: number | null;
}

export interface LogEvent {
  id: number;
  tracker_id: string;
  label: string;
  ts: string;
  kind: 'enter' | 'exit' | 'landing' | 'takeoff';
  airport: string;
  aircraft: string;
  distance_nm: number | null;
  alt_ft: number | null;
}

export type ProviderConfig =
  | { type: 'local'; url: string; enabled: boolean }
  | { type: 'adsb_lol'; enabled: boolean }
  | { type: 'airplanes_live'; enabled: boolean }
  | { type: 'adsb_fi'; enabled: boolean }
  | { type: 'open_sky'; client_id: string; enabled: boolean }
  | { type: 'adsb_exchange'; enabled: boolean }
  | { type: 'mock'; enabled: boolean };

export interface Settings {
  providers: ProviderConfig[];
  poll_interval_s: number;
  distance_unit: DistUnit;
  theme: 'system' | 'light' | 'dark';
  notifications: {
    ntfy: { enabled: boolean; server: string; topic: string };
    email: { enabled: boolean; host: string; port: number; username: string; security: string; from: string; to: string };
    webhook: { enabled: boolean };
    twilio: { enabled: boolean; account_sid: string; from: string; to: string };
    sound: boolean;
    desktop: boolean;
  };
  blocklist: string[];
  default_channels: ChannelKind[];
}

export interface ProviderHealth {
  id: string;
  ok: boolean | null;
  consecutive_failures: number;
  last_error: string | null;
  last_ok_secs_ago: number | null;
  backoff_secs: number | null;
}

export class ApiError extends Error {}

async function req<T>(method: string, url: string, body?: unknown): Promise<T> {
  const r = await fetch(url, {
    method,
    headers: body !== undefined ? { 'content-type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!r.ok) {
    let msg = `HTTP ${r.status}`;
    try {
      msg = (await r.json()).error ?? msg;
    } catch {}
    throw new ApiError(msg);
  }
  if (r.status === 204) return undefined as T;
  return r.json();
}

export const api = {
  info: () => req<{ version: string; mode: 'server' | 'desktop'; auth: boolean; airports: number }>('GET', '/api/info'),
  trackers: () => req<Tracker[]>('GET', '/api/trackers'),
  createTracker: (t: Partial<Tracker>) => req<Tracker>('POST', '/api/trackers', t),
  updateTracker: (t: Tracker) => req<Tracker>('PUT', `/api/trackers/${t.id}`, t),
  deleteTracker: (id: string) => req<void>('DELETE', `/api/trackers/${id}`),
  testAlert: (id: string) => req<{ results: { channel: string; ok: boolean; error: string | null }[] }>('POST', `/api/trackers/${id}/test-alert`),
  testChannel: (channel: ChannelKind) => req<{ results: { channel: string; ok: boolean; error: string | null }[] }>('POST', '/api/notify/test', { channel }),
  state: () => req<{ states: TrackerState[]; errors: Record<string, string> }>('GET', '/api/state'),
  events: (tracker?: string, limit = 300) => req<LogEvent[]>('GET', `/api/events?limit=${limit}${tracker ? `&tracker=${tracker}` : ''}`),
  settings: () => req<Settings>('GET', '/api/settings'),
  saveSettings: (s: Settings) => req<Settings>('PUT', '/api/settings', s),
  secrets: () => req<Record<string, boolean>>('GET', '/api/secrets'),
  setSecret: (name: string, value: string) => req<void>('PUT', `/api/secrets/${name}`, { value }),
  deleteSecret: (name: string) => req<void>('DELETE', `/api/secrets/${name}`),
  airports: (q: string) => req<Airport[]>('GET', `/api/airports?q=${encodeURIComponent(q)}&limit=8`),
  airport: (ident: string) => req<Airport>('GET', `/api/airports/${encodeURIComponent(ident)}`),
  traffic: (lat: number, lon: number, radius: number) =>
    req<{ provider: string; aircraft: Aircraft[] }>('GET', `/api/traffic?lat=${lat}&lon=${lon}&radius=${radius}`),
  lookup: (q: string) => req<{ target: Target | null; hex: string | null; n_number: string | null }>('GET', `/api/lookup?q=${encodeURIComponent(q)}`),
  providers: () => req<ProviderHealth[]>('GET', '/api/providers'),
  pollNow: () => req<void>('POST', '/api/poll'),
};
