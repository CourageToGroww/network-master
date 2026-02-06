import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import type { ClientMessage, ServerMessage } from './types';
import { handleMessage } from './messageHandlers';

type WSStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting';

interface WSContextValue {
  send: (msg: ClientMessage) => void;
  status: WSStatus;
  subscribe: (targetIds: string[]) => void;
  unsubscribe: (targetIds: string[]) => void;
}

const WSContext = createContext<WSContextValue>(null as unknown as WSContextValue);

export function useWS() {
  const ctx = useContext(WSContext);
  if (!ctx) throw new Error('useWS must be used within WebSocketProvider');
  return ctx;
}

const WS_URL = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws/live`;

export function WebSocketProvider({ children }: { children: ReactNode }) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const reconnectAttempt = useRef(0);
  const subscriptions = useRef(new Set<string>());
  const mountedRef = useRef(false);
  const [status, setStatus] = useState<WSStatus>('connecting');

  const connect = useCallback(() => {
    if (!mountedRef.current) return;

    // Clean up any existing connection first
    if (wsRef.current) {
      const old = wsRef.current;
      old.onopen = null;
      old.onmessage = null;
      old.onclose = null;
      old.onerror = null;
      old.close();
      wsRef.current = null;
    }

    try {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.onopen = () => {
        if (!mountedRef.current) { ws.close(); return; }
        setStatus('connected');
        reconnectAttempt.current = 0;
        if (subscriptions.current.size > 0) {
          ws.send(JSON.stringify({
            type: 'Subscribe',
            data: { target_ids: Array.from(subscriptions.current) },
          }));
        }
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data) as ServerMessage;
          handleMessage(msg);
        } catch {
          // Ignore parse errors
        }
      };

      ws.onclose = () => {
        if (!mountedRef.current) return;
        setStatus('reconnecting');
        const delay = Math.min(1000 * 2 ** reconnectAttempt.current, 30000);
        reconnectTimer.current = setTimeout(() => {
          reconnectAttempt.current++;
          connect();
        }, delay);
      };

      ws.onerror = () => {
        ws.close();
      };
    } catch {
      if (mountedRef.current) setStatus('disconnected');
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    connect();
    return () => {
      mountedRef.current = false;
      clearTimeout(reconnectTimer.current);
      if (wsRef.current) {
        const ws = wsRef.current;
        ws.onopen = null;
        ws.onmessage = null;
        ws.onclose = null;
        ws.onerror = null;
        ws.close();
        wsRef.current = null;
      }
    };
  }, [connect]);

  const send = useCallback((msg: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  const subscribe = useCallback((targetIds: string[]) => {
    targetIds.forEach((id) => subscriptions.current.add(id));
    send({ type: 'Subscribe', data: { target_ids: targetIds } });
  }, [send]);

  const unsubscribe = useCallback((targetIds: string[]) => {
    targetIds.forEach((id) => subscriptions.current.delete(id));
    send({ type: 'Unsubscribe', data: { target_ids: targetIds } });
  }, [send]);

  return (
    <WSContext.Provider value={{ send, status, subscribe, unsubscribe }}>
      {children}
    </WSContext.Provider>
  );
}
