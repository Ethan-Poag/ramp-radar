import { api, type Airport, type LogEvent, type Settings, type Tracker, type TrackerState, type ProviderHealth, type UpdateInfo } from './api';

export const NEW = 'new';

class Store {
  trackers = $state<Tracker[]>([]);
  states = $state<Record<string, TrackerState>>({});
  errors = $state<Record<string, string>>({});
  settings = $state<Settings | null>(null);
  info = $state<{ version: string; mode: 'server' | 'desktop'; auth: boolean } | null>(null);
  providers = $state<ProviderHealth[]>([]);
  airports = $state<Record<string, Airport>>({});
  events = $state<LogEvent[]>([]);
  selected = $state<string | null>(null); // tracker id | 'new' | null
  connected = $state(false);
  toasts = $state<{ id: number; text: string; kind: 'ok' | 'err' | 'alert' }[]>([]);
  tab = $state<'trackers' | 'map' | 'log' | 'settings'>('trackers');
  update = $state<UpdateInfo & { checking: boolean; installing: boolean; error: string; dismissed: boolean }>({
    available: false,
    checking: false,
    installing: false,
    error: '',
    dismissed: false,
  });
  private tid = 0;

  get unit() {
    return this.settings?.distance_unit ?? 'nm';
  }
  get selectedTracker(): Tracker | null {
    return this.trackers.find((t) => t.id === this.selected) ?? null;
  }

  toast(text: string, kind: 'ok' | 'err' | 'alert' = 'ok', ms = 4500) {
    const id = ++this.tid;
    this.toasts.push({ id, text, kind });
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), ms);
  }

  async ensureAirport(ident: string) {
    if (this.airports[ident]) return;
    try {
      this.airports[ident] = await api.airport(ident);
    } catch {}
  }

  async loadTrackers() {
    this.trackers = await api.trackers();
    for (const t of this.trackers) this.ensureAirport(t.airport);
  }
  async loadSettings() {
    this.settings = await api.settings();
    applyTheme(this.settings.theme);
  }
  async loadEvents() {
    this.events = await api.events(undefined, 500);
  }

  async init() {
    this.info = await api.info();
    await Promise.all([this.loadTrackers(), this.loadSettings(), this.loadEvents()]);
    api.providers().then((p) => (this.providers = p)).catch(() => {});
    setInterval(() => api.providers().then((p) => (this.providers = p)).catch(() => {}), 10_000);
    this.connect();
    if (this.info?.mode === 'desktop') this.checkUpdates(true);
  }

  async checkUpdates(silent = false) {
    if (this.info?.mode !== 'desktop') return;
    this.update.checking = true;
    this.update.error = '';
    try {
      const r = await api.checkUpdate();
      this.update.available = r.available;
      this.update.current_version = r.current_version;
      this.update.version = r.version;
      this.update.notes = r.notes;
      this.update.checking = false;
      if (!r.available) this.update.dismissed = false;
      if (!silent) {
        this.toast(r.available ? `Ramp Radar ${r.version} is available` : 'You are up to date');
      }
    } catch (e: any) {
      this.update.checking = false;
      this.update.error = String(e.message ?? e);
      if (!silent) this.toast(this.update.error, 'err');
    }
  }

  async installUpdate() {
    if (this.info?.mode !== 'desktop' || this.update.installing) return;
    this.update.installing = true;
    this.update.error = '';
    try {
      await api.installUpdate();
      this.toast('Update installed — restarting…');
    } catch (e: any) {
      this.update.installing = false;
      this.update.error = String(e.message ?? e);
      this.toast(this.update.error, 'err');
    }
  }

  private connect() {
    const es = new EventSource('/api/stream');
    es.onopen = () => (this.connected = true);
    es.onerror = () => (this.connected = false);
    es.addEventListener('state', (e) => {
      const m = JSON.parse((e as MessageEvent).data);
      const next: Record<string, TrackerState> = {};
      for (const s of m.states as TrackerState[]) next[s.tracker_id] = s;
      this.states = next;
      this.errors = m.errors;
    });
    es.addEventListener('event', (e) => {
      const m = JSON.parse((e as MessageEvent).data);
      this.events = [m.event, ...this.events].slice(0, 500);
    });
    es.addEventListener('alert', (e) => {
      const { alert } = JSON.parse((e as MessageEvent).data);
      this.toast(`${alert.title}`, 'alert', 8000);
      if (alert.sound) beep();
      // Browser (server-mode) notifications; the desktop shell shows native ones itself.
      if (alert.desktop && this.info?.mode === 'server' && 'Notification' in window && Notification.permission === 'granted') {
        new Notification(alert.title, { body: alert.body });
      }
    });
  }
}

export const store = new Store();

export function applyTheme(t: 'system' | 'light' | 'dark') {
  document.documentElement.dataset.theme = t;
}

let ctx: AudioContext | null = null;
export function beep() {
  try {
    ctx ??= new AudioContext();
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    o.connect(g);
    g.connect(ctx.destination);
    o.type = 'sine';
    o.frequency.setValueAtTime(880, ctx.currentTime);
    o.frequency.setValueAtTime(1175, ctx.currentTime + 0.15);
    g.gain.setValueAtTime(0.0001, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.25, ctx.currentTime + 0.02);
    g.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.5);
    o.start();
    o.stop(ctx.currentTime + 0.55);
  } catch {}
}
