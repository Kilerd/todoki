/**
 * useEventStream Hook Tests
 *
 * Tests for WebSocket event streaming hook functionality.
 * Note: These tests require a running server and are primarily for documentation.
 * Run manually or use Playwright for true E2E testing.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useEventStream } from './useEventStream';

// Mock WebSocket
class MockWebSocket {
  public onopen: ((event: Event) => void) | null = null;
  public onmessage: ((event: MessageEvent) => void) | null = null;
  public onerror: ((event: Event) => void) | null = null;
  public onclose: ((event: CloseEvent) => void) | null = null;
  public readyState: number = WebSocket.CONNECTING;

  constructor(public url: string) {
    // Simulate async connection
    setTimeout(() => {
      this.readyState = WebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    }, 10);
  }

  send(data: string) {
    // Mock implementation
  }

  close() {
    this.readyState = WebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }
}

describe('useEventStream', () => {
  beforeEach(() => {
    // Mock WebSocket globally
    global.WebSocket = MockWebSocket as any;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('should initialize with empty events and disconnected state', () => {
    const { result } = renderHook(() => useEventStream());

    expect(result.current.events).toEqual([]);
    expect(result.current.isConnected).toBe(false);
    expect(result.current.isReplaying).toBe(false);
    expect(result.current.error).toBe(null);
  });

  it('should connect to WebSocket on mount', async () => {
    const { result } = renderHook(() => useEventStream({ kinds: ['task.*'] }));

    // Wait for connection
    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });
  });

  it('should build correct WebSocket URL with parameters', () => {
    const mockWebSocket = vi.spyOn(global, 'WebSocket');

    renderHook(() =>
      useEventStream({
        kinds: ['task.*', 'agent.*'],
        cursor: 100,
        agentId: 'agent-123',
        taskId: 'task-456',
        token: 'test-token',
      })
    );

    expect(mockWebSocket).toHaveBeenCalledWith(
      expect.stringContaining('kinds=task.*,agent.*')
    );
    expect(mockWebSocket).toHaveBeenCalledWith(expect.stringContaining('cursor=100'));
    expect(mockWebSocket).toHaveBeenCalledWith(
      expect.stringContaining('agent_id=agent-123')
    );
    expect(mockWebSocket).toHaveBeenCalledWith(
      expect.stringContaining('task_id=task-456')
    );
    expect(mockWebSocket).toHaveBeenCalledWith(expect.stringContaining('token=test-token'));
  });

  it('should handle incoming events', async () => {
    const { result } = renderHook(() => useEventStream());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Simulate receiving an event
    const mockEvent = {
      type: 'event',
      cursor: 1,
      kind: 'task.created',
      time: '2026-02-27T10:00:00Z',
      agent_id: 'agent-123',
      session_id: null,
      task_id: 'task-456',
      data: { content: 'Test task' },
    };

    // Trigger onmessage
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.onmessage(new MessageEvent('message', { data: JSON.stringify(mockEvent) }));

    await waitFor(() => {
      expect(result.current.events).toHaveLength(1);
      expect(result.current.events[0].kind).toBe('task.created');
      expect(result.current.events[0].cursor).toBe(1);
    });
  });

  it('should set isReplaying when cursor is provided', () => {
    const { result } = renderHook(() => useEventStream({ cursor: 100 }));

    expect(result.current.isReplaying).toBe(true);
  });

  it('should clear isReplaying on replay_complete', async () => {
    const { result } = renderHook(() => useEventStream({ cursor: 100 }));

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    expect(result.current.isReplaying).toBe(true);

    // Simulate replay_complete
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.onmessage(
      new MessageEvent('message', {
        data: JSON.stringify({ type: 'replay_complete', count: 50 }),
      })
    );

    await waitFor(() => {
      expect(result.current.isReplaying).toBe(false);
    });
  });

  it('should handle server errors', async () => {
    const { result } = renderHook(() => useEventStream());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Simulate error message
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.onmessage(
      new MessageEvent('message', {
        data: JSON.stringify({
          type: 'error',
          message: 'Stream lagged by 10 events',
        }),
      })
    );

    await waitFor(() => {
      expect(result.current.error).toBe('Stream lagged by 10 events');
    });
  });

  it('should reconnect with exponential backoff', async () => {
    const { result } = renderHook(() =>
      useEventStream({ autoReconnect: true, maxReconnectAttempts: 3 })
    );

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Simulate disconnect
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.close();

    expect(result.current.isConnected).toBe(false);

    // Should attempt reconnect after 1 second (first attempt)
    vi.advanceTimersByTime(1000);

    await waitFor(() => {
      expect(global.WebSocket).toHaveBeenCalledTimes(2);
    });
  });

  it('should stop reconnecting after max attempts', async () => {
    const { result } = renderHook(() =>
      useEventStream({ autoReconnect: true, maxReconnectAttempts: 2 })
    );

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Simulate multiple disconnects
    for (let i = 0; i < 3; i++) {
      const ws = (global.WebSocket as any).mock.results[i].value;
      ws.close();

      // Advance timer for reconnect
      vi.advanceTimersByTime(Math.pow(2, i) * 1000);
      await waitFor(() => {});
    }

    // Should have stopped after 2 attempts (3 total connections)
    expect(global.WebSocket).toHaveBeenCalledTimes(3);
  });

  it('should manually reconnect and reset attempt counter', async () => {
    const { result } = renderHook(() => useEventStream());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Disconnect
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.close();

    // Manual reconnect
    result.current.reconnect();

    await waitFor(() => {
      expect(global.WebSocket).toHaveBeenCalledTimes(2);
    });
  });

  it('should clear events', async () => {
    const { result } = renderHook(() => useEventStream());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Add some events
    const ws = (global.WebSocket as any).mock.results[0].value;
    ws.onmessage(
      new MessageEvent('message', {
        data: JSON.stringify({
          type: 'event',
          cursor: 1,
          kind: 'task.created',
          time: '2026-02-27T10:00:00Z',
          agent_id: 'agent-123',
          data: {},
        }),
      })
    );

    await waitFor(() => {
      expect(result.current.events).toHaveLength(1);
    });

    // Clear events
    result.current.clearEvents();

    expect(result.current.events).toEqual([]);
  });

  it('should disconnect on unmount', async () => {
    const { result, unmount } = renderHook(() => useEventStream());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    const ws = (global.WebSocket as any).mock.results[0].value;
    const closeSpy = vi.spyOn(ws, 'close');

    unmount();

    expect(closeSpy).toHaveBeenCalled();
  });
});
