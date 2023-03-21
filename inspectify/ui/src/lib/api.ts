import { useEffect, useState } from "react";
import type { QueryClient } from "react-query";
import {
  Analysis,
  AnalysisRequest,
  AnalysisResponse,
  CompilationStatus,
  GraphRequest,
  GraphResponse,
} from "./types";

const request = async <T>(
  signal: AbortSignal | null,
  name: string,
  body: unknown
): Promise<T> => {
  const headers = new Headers();
  headers.append("Content-Type", "application/json");

  const req = await fetch(`http://localhost:3000/${name}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
    signal,
  });
  return req.json();
};

export const analyze = (
  signal: AbortSignal | void,
  req: {
    analysis: Analysis;
    input: string;
    src: string;
  }
): Promise<AnalysisResponse> => {
  const internalRequest = {
    analysis: Analysis[req.analysis],
    input: req.input,
    src: req.src,
  } satisfies AnalysisRequest;
  return request(signal ?? null, "analyze", internalRequest);
};

export const graph = (
  signal: AbortSignal | undefined,
  req: GraphRequest
): Promise<GraphResponse> => request(signal ?? null, "graph", req);

export const useCompilationStatus = ({
  queryClient,
}: {
  queryClient: QueryClient;
}) => {
  const [state, setState] = useState<CompilationStatus | null>(null);

  useEffect(() => {
    let ws: WebSocket | null = null;
    let timeout: number = 0;

    const connect = () => {
      ws = new WebSocket("ws://localhost:3000/compilation-ws");

      ws.onopen = () => {
        console.info("WebSocket opened up!");
      };

      ws.onclose = () => {
        console.info("WebSocket closed!");

        setState({
          compiled_at: 0,
          state: { type: "Compiling", content: void 0 },
        });

        window.clearTimeout(timeout);
        timeout = window.setTimeout(connect, 1000);
      };

      ws.onmessage = (rawMsg) => {
        if (!(typeof rawMsg.data == "string")) return;
        const msg = JSON.parse(rawMsg.data);
        setState(msg);
      };

      return () => {
        if (ws) ws.close();
        if (timeout) window.clearTimeout(timeout);
      };
    };

    const close = connect();

    return () => {
      close();
    };
  }, [queryClient]);

  return state;
};
