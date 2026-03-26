"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import type { DashboardUpdate } from "@/lib/types";

export function useWebSocket() {
  const [data, setData] = useState<DashboardUpdate | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<NodeJS.Timeout | null>(null);

  const connect = useCallback(() => {
    if (typeof window === "undefined") return;

    // Connect directly to backend WebSocket (port 3456)
    // Next.js rewrites don't handle WebSocket protocol upgrades properly
    const wsUrl = process.env.NODE_ENV === "production"
      ? `wss://${window.location.host}/ws`
      : "ws://localhost:3456/ws";
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setIsConnected(true);
    };

    ws.onmessage = (event) => {
      try {
        const update: DashboardUpdate = JSON.parse(event.data);
        setData(update);
      } catch {
        // ignore malformed messages
      }
    };

    ws.onclose = () => {
      setIsConnected(false);
      wsRef.current = null;
      reconnectTimeout.current = setTimeout(connect, 2000);
    };

    ws.onerror = () => {
      ws.close();
    };
  }, []);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimeout.current) clearTimeout(reconnectTimeout.current);
      if (wsRef.current) wsRef.current.close();
    };
  }, [connect]);

  return { data, isConnected };
}
