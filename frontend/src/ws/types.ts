import type { LiveProcessTrafficUpdate, LiveTraceUpdate, UpdateProgressData } from '../types';

export type ServerMessage =
  | { type: 'live_trace'; data: LiveTraceUpdate }
  | { type: 'alert_fired'; data: { alert_event_id: string; rule_name: string; message: string } }
  | { type: 'agent_status'; data: { agent_id: string; agent_name: string; is_online: boolean } }
  | { type: 'route_change'; data: { target_id: string; session_id: string; hops_changed: number } }
  | { type: 'update_status'; data: UpdateProgressData }
  | { type: 'process_traffic'; data: LiveProcessTrafficUpdate };

export type ClientMessage =
  | { type: 'Subscribe'; data: { target_ids: string[] } }
  | { type: 'Unsubscribe'; data: { target_ids: string[] } }
  | { type: 'SubscribeTraffic'; data: { agent_ids: string[] } }
  | { type: 'UnsubscribeTraffic'; data: { agent_ids: string[] } };
