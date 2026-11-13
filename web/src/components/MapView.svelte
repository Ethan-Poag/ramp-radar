<script lang="ts">
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { onMount } from 'svelte';
  import { api, type Aircraft } from '../lib/api';
  import { store } from '../lib/store.svelte';
  import { circle, fmtAlt, fmtDist, STATUS_LABEL } from '../lib/format';

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let ready = $state(false);
  let showTraffic = $state(true);
  let traffic = $state<Aircraft[]>([]);
  let trafficSrc = $state('');
  let trafficErr = $state('');
  let glError = $state('');

  const focusTracker = $derived(store.selectedTracker ?? store.trackers[0] ?? null);
  const focusAp = $derived(focusTracker ? store.airports[focusTracker.airport] : (store.airports['KBOS'] ?? null));
  const dark = () =>
    document.documentElement.dataset.theme === 'dark' ||
    (document.documentElement.dataset.theme === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);

  const fc = (features: any[]) => ({ type: 'FeatureCollection' as const, features });
  const empty = fc([]);

  function planeImage(): ImageData {
    const c = document.createElement('canvas');
    c.width = c.height = 64;
    const g = c.getContext('2d')!;
    g.fillStyle = '#fff';
    g.beginPath();
    // nose up airliner silhouette
    const p: [number, number][] = [[32, 4], [36, 12], [36, 26], [60, 42], [60, 48], [36, 40], [36, 52], [44, 58], [44, 62], [32, 59], [20, 62], [20, 58], [28, 52], [28, 40], [4, 48], [4, 42], [28, 26], [28, 12]];
    p.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
    g.closePath();
    g.fill();
    return g.getImageData(0, 0, 64, 64);
  }

  onMount(() => {
    if (!store.airports['KBOS']) store.ensureAirport('KBOS');
    try { map = new maplibregl.Map({
      container: el,
      center: [focusAp?.lon ?? -71.0052, focusAp?.lat ?? 42.3643],
      zoom: 9,
      attributionControl: { compact: true },
      style: {
        version: 8,
        glyphs: 'https://protomaps.github.io/basemaps-assets/fonts/{fontstack}/{range}.pbf',
        sources: {
          osm: {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            maxzoom: 19,
            attribution: '© <a href="https://www.openstreetmap.org/copyright" target="_blank">OpenStreetMap</a> contributors',
          },
        },
        layers: [{ id: 'osm', type: 'raster', source: 'osm', paint: dark() ? { 'raster-brightness-max': 0.45, 'raster-saturation': -0.4 } : {} }],
      },
    }); } catch (e: any) { glError = 'Could not start the map: ' + (e?.message ?? e); return; }
    map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-right');
    map.on('load', () => {
      const m = map!;
      m.addImage('plane', planeImage(), { sdf: true });
      for (const id of ['rings', 'trails', 'tracked', 'traffic', 'airports']) m.addSource(id, { type: 'geojson', data: empty });
      m.addLayer({ id: 'rings-fill', type: 'fill', source: 'rings', paint: { 'fill-color': ['get', 'color'], 'fill-opacity': 0.07 } });
      m.addLayer({ id: 'rings-line', type: 'line', source: 'rings', paint: { 'line-color': ['get', 'color'], 'line-width': 2, 'line-dasharray': [3, 2] } });
      m.addLayer({ id: 'trails', type: 'line', source: 'trails', paint: { 'line-color': ['get', 'color'], 'line-width': 3, 'line-opacity': 0.8 } });
      m.addLayer({ id: 'airports', type: 'circle', source: 'airports', paint: { 'circle-radius': 6, 'circle-color': '#0f172a', 'circle-stroke-color': '#fff', 'circle-stroke-width': 2 } });
      m.addLayer({ id: 'airports-label', type: 'symbol', source: 'airports', layout: { 'text-field': ['get', 'name'], 'text-font': ['Noto Sans Regular'], 'text-size': 12, 'text-offset': [0, 1.2], 'text-anchor': 'top' }, paint: { 'text-color': dark() ? '#e5eaf3' : '#0f172a', 'text-halo-color': dark() ? '#0b1220' : '#fff', 'text-halo-width': 1.5 } });
      m.addLayer({ id: 'traffic', type: 'symbol', source: 'traffic', layout: { 'icon-image': 'plane', 'icon-size': 0.38, 'icon-rotate': ['get', 'track'], 'icon-rotation-alignment': 'map', 'icon-allow-overlap': true, 'text-field': ['get', 'label'], 'text-font': ['Noto Sans Regular'], 'text-size': 10, 'text-offset': [0, 1.4], 'text-optional': true }, paint: { 'icon-color': '#64748b', 'icon-halo-color': '#fff', 'icon-halo-width': 1, 'text-color': dark() ? '#cbd5e1' : '#334155', 'text-halo-color': dark() ? '#0b1220' : '#fff', 'text-halo-width': 1.2 } });
      m.addLayer({ id: 'tracked', type: 'symbol', source: 'tracked', layout: { 'icon-image': 'plane', 'icon-size': 0.6, 'icon-rotate': ['get', 'track'], 'icon-rotation-alignment': 'map', 'icon-allow-overlap': true, 'text-field': ['get', 'label'], 'text-font': ['Noto Sans Regular'], 'text-size': 12, 'text-offset': [0, 1.8], 'text-allow-overlap': true }, paint: { 'icon-color': ['get', 'color'], 'icon-halo-color': '#fff', 'icon-halo-width': 2, 'text-color': dark() ? '#fff' : '#0f172a', 'text-halo-color': dark() ? '#0b1220' : '#fff', 'text-halo-width': 1.5 } });
      for (const layer of ['tracked', 'traffic']) {
        m.on('click', layer, (e) => {
          const f = e.features?.[0];
          if (!f) return;
          const div = document.createElement('div');
          div.style.cssText = 'font:13px system-ui;color:#0f172a;line-height:1.4';
          const b = document.createElement('b');
          b.textContent = String(f.properties?.label ?? '');
          div.append(b, document.createElement('br'), document.createTextNode(String(f.properties?.detail ?? '')));
          new maplibregl.Popup({ closeButton: false }).setLngLat((f.geometry as any).coordinates).setDOMContent(div).addTo(m);
        });
        m.on('mouseenter', layer, () => (m.getCanvas().style.cursor = 'pointer'));
        m.on('mouseleave', layer, () => (m.getCanvas().style.cursor = ''));
      }
      ready = true;
    });
    return () => map?.remove();
  });

  // static layers: rings + airports
  $effect(() => {
    if (!ready) return;
    const rings: any[] = [];
    const aps = new Map<string, any>();
    for (const t of store.trackers) {
      const ap = store.airports[t.airport];
      if (!ap) continue;
      if (t.enabled) {
        const nm = t.radius.unit === 'nm' ? t.radius.value : t.radius.unit === 'mi' ? t.radius.value * 0.868976 : t.radius.value * 0.539957;
        rings.push({ type: 'Feature', properties: { color: t.color }, geometry: { type: 'Polygon', coordinates: [circle(ap.lat, ap.lon, nm)] } });
      }
      aps.set(ap.ident, { type: 'Feature', properties: { name: ap.iata ? `${ap.ident} · ${ap.iata}` : ap.ident }, geometry: { type: 'Point', coordinates: [ap.lon, ap.lat] } });
    }
    if (focusAp && !aps.has(focusAp.ident)) aps.set(focusAp.ident, { type: 'Feature', properties: { name: focusAp.ident }, geometry: { type: 'Point', coordinates: [focusAp.lon, focusAp.lat] } });
    (map!.getSource('rings') as maplibregl.GeoJSONSource).setData(fc(rings));
    (map!.getSource('airports') as maplibregl.GeoJSONSource).setData(fc([...aps.values()]));
  });

  // live layers: tracked aircraft + trails
  $effect(() => {
    if (!ready) return;
    const pts: any[] = [];
    const trails: any[] = [];
    for (const t of store.trackers) {
      const s = store.states[t.id];
      if (!s || s.lat == null || s.lon == null || !t.enabled || s.status === 'not_seen') continue;
      const name = s.reg ?? s.callsign ?? t.target.value.toUpperCase();
      pts.push({
        type: 'Feature',
        properties: {
          color: t.color, track: s.track ?? 0, label: name,
          detail: `${t.label} · ${STATUS_LABEL[s.status]} · ${fmtDist(s.distance_nm, store.unit)} from ${t.airport} · ${fmtAlt(s.alt_ft)}`,
        },
        geometry: { type: 'Point', coordinates: [s.lon, s.lat] },
      });
      if (s.trail.length > 1) trails.push({ type: 'Feature', properties: { color: t.color }, geometry: { type: 'LineString', coordinates: s.trail.map((p) => [p.lon, p.lat]) } });
    }
    (map!.getSource('tracked') as maplibregl.GeoJSONSource).setData(fc(pts));
    (map!.getSource('trails') as maplibregl.GeoJSONSource).setData(fc(trails));
  });

  // nearby traffic
  $effect(() => {
    if (!ready) return;
    const tracked = new Set(Object.values(store.states).map((s) => s.hex).filter(Boolean));
    const feats = showTraffic
      ? traffic.filter((a) => a.lat != null && a.lon != null && !tracked.has(a.hex)).map((a) => ({
          type: 'Feature',
          properties: { track: a.track ?? 0, label: (a.callsign ?? a.reg ?? a.hex).trim(), detail: `${a.type_code ?? ''} ${fmtAlt(a.on_ground ? 0 : a.alt_ft)} ${a.gs_kt != null ? Math.round(a.gs_kt) + ' kt' : ''}` },
          geometry: { type: 'Point', coordinates: [a.lon!, a.lat!] },
        }))
      : [];
    (map!.getSource('traffic') as maplibregl.GeoJSONSource).setData(fc(feats));
  });

  async function loadTraffic() {
    if (!showTraffic || !focusAp) return;
    try {
      const r = await api.traffic(focusAp.lat, focusAp.lon, 40);
      traffic = r.aircraft;
      trafficSrc = r.provider;
      trafficErr = '';
    } catch (e: any) {
      trafficErr = e.message;
    }
  }
  $effect(() => {
    focusAp?.ident;
    showTraffic;
    traffic = [];
    loadTraffic();
    const i = setInterval(loadTraffic, 10_000);
    return () => clearInterval(i);
  });

  // fly to the focused airport when it changes
  let lastFocus = '';
  $effect(() => {
    if (!ready || !focusAp || lastFocus === focusAp.ident) return;
    lastFocus = focusAp.ident;
    const r = focusTracker ? Math.max(focusTracker.radius.value, 1) : 10;
    const nmR = focusTracker ? (focusTracker.radius.unit === 'nm' ? r : focusTracker.radius.unit === 'mi' ? r * 0.868976 : r * 0.539957) : 10;
    const ring = circle(focusAp.lat, focusAp.lon, Math.max(nmR * 1.6, 8));
    const b = new maplibregl.LngLatBounds(ring[0], ring[0]);
    ring.forEach((p) => b.extend(p));
    map!.fitBounds(b, { padding: 30, duration: 600, maxZoom: 13 });
  });
</script>

<div class="wrap">
  <div class="map" bind:this={el}></div>
  {#if glError}<div class="card gl">{glError}</div>{/if}
  <div class="panel card">
    <div class="small"><b>{focusAp ? focusAp.ident : '—'}</b> {focusAp?.name ?? ''}</div>
    <label class="check small"><input type="checkbox" bind:checked={showTraffic} /> Nearby traffic {#if showTraffic && traffic.length}<span class="muted">({traffic.length})</span>{/if}</label>
    {#if trafficErr}<div class="small" style="color:var(--bad)">{trafficErr}</div>{/if}
    <div class="legend small muted">
      {#each store.trackers as t}<span class:dim={!t.enabled}><i style="background:{t.color}"></i>{store.states[t.id]?.reg ?? t.target.value.toUpperCase()}</span>{/each}
    </div>
    {#if trafficSrc}<div class="small muted">Traffic: {trafficSrc}{#if trafficSrc === 'adsb.lol'} (ODbL){/if}</div>{/if}
  </div>
</div>

<style>
  .wrap { position: relative; height: 100%; min-height: 360px; }
  .map { position: absolute; inset: 0; }
  .panel { position: absolute; left: .75rem; top: .75rem; padding: .6rem .8rem; max-width: min(18rem, 70vw); z-index: 5; }
  .legend { display: flex; flex-wrap: wrap; gap: .2rem .8rem; margin-top: .3rem; }
  .legend i { display: inline-block; width: 9px; height: 9px; border-radius: 50%; margin-right: .3rem; }
  .gl { position: absolute; inset: 40% 10% auto; padding: 1rem; text-align: center; z-index: 6; }
  .dim { opacity: .45; }
</style>
