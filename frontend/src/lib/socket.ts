import { dashboard } from './state.svelte';
import type { DashboardMessage } from './types';

const MIN_RECONNECT_DELAY_MS = 500;
const MAX_RECONNECT_DELAY_MS = 10_000;

/**
 * Opens the dashboard's WebSocket connection and keeps it alive,
 * reconnecting with exponential backoff if it drops. Every message is
 * applied to the shared {@link dashboard} store.
 */
export function connect(): void {
  let delay = MIN_RECONNECT_DELAY_MS;

  const open = () => {
    const url = new URL('/dashboard/ws', window.location.href);
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    const socket = new WebSocket(url);

    socket.addEventListener('open', () => {
      dashboard.connected = true;
      delay = MIN_RECONNECT_DELAY_MS;
    });

    socket.addEventListener('message', (event) => {
      const message: DashboardMessage = JSON.parse(event.data);
      dashboard.applyMessage(message);
    });

    socket.addEventListener('close', () => {
      dashboard.connected = false;
      setTimeout(open, delay);
      delay = Math.min(delay * 2, MAX_RECONNECT_DELAY_MS);
    });

    socket.addEventListener('error', () => socket.close());
  };

  open();
}
