import {
  AnalysisRequest,
  AnalysisResponse,
  CompilationStatus,
  GraphRequest,
  GraphResponse,
} from "./api-types";

export const analyze = (
  req: AnalysisRequest
): { abort: () => void; promise: Promise<AnalysisResponse> } => {
  const headers = new Headers();
  headers.append("Content-Type", "application/json");

  const controller = new AbortController();

  const promise = fetch("http://localhost:3000/analyze", {
    method: "POST",
    headers,
    body: JSON.stringify(req),
    signal: controller.signal,
  }).then((res) => res.json());

  return { abort: () => controller.abort(), promise };
};

export const graph = (
  req: GraphRequest
): { abort: () => void; promise: Promise<GraphResponse> } => {
  const headers = new Headers();
  headers.append("Content-Type", "application/json");

  const controller = new AbortController();

  const promise = fetch("http://localhost:3000/graph", {
    method: "POST",
    headers,
    body: JSON.stringify(req),
    signal: controller.signal,
  }).then((res) => res.json());

  return { abort: () => controller.abort(), promise };
};

export const compilationStatus = (): {
  abort: () => void;
  promise: Promise<CompilationStatus>;
} => {
  const controller = new AbortController();

  const promise = fetch("http://localhost:3000/compilation-status", {
    method: "GET",
    signal: controller.signal,
  }).then((res) => res.json());

  return { abort: () => controller.abort(), promise };
};
