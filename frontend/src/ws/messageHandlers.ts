import type { ServerMessage } from './types';
import { useTraceStore } from '../stores/traceStore';
import { useAgentStore } from '../stores/agentStore';
import { useTrafficStore } from '../stores/trafficStore';
import { queryClient } from '../api/client';

export function handleMessage(msg: ServerMessage): void {
  switch (msg.type) {
    case 'live_trace': {
      const { agent_id, target_id, sent_at, hops } = msg.data;
      useTraceStore.getState().pushRound(agent_id, target_id, sent_at, hops);
      break;
    }

    case 'agent_status': {
      useAgentStore.getState().setAgentOnline(msg.data.agent_id, msg.data.is_online);
      queryClient.invalidateQueries({ queryKey: ['agents'] });
      break;
    }

    case 'alert_fired': {
      queryClient.invalidateQueries({ queryKey: ['alert-events'] });
      break;
    }

    case 'route_change': {
      break;
    }

    case 'update_status': {
      queryClient.invalidateQueries({ queryKey: ['agents'] });
      break;
    }

    case 'process_traffic': {
      useTrafficStore.getState().pushTraffic(msg.data);
      break;
    }
  }
}
