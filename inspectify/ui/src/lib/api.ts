import type * as wasm from "../../../wasm/pkg";
import {
  Analysis,
  AnalysisRequest,
  AnalysisResponse,
  CompilationStatus,
  GraphRequest,
  GraphResponse,
} from "./types";

export const analyze = (req: {
  analysis: wasm.Analysis;
  input: string;
  src: string;
}): { abort: () => void; promise: Promise<AnalysisResponse> } => {
  const internalRequest = {
    analysis: Analysis[req.analysis],
    input: req.input,
    src: req.src,
  } satisfies AnalysisRequest;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");

  const controller = new AbortController();

  const promise = fetch("http://localhost:3000/analyze", {
    method: "POST",
    headers,
    body: JSON.stringify(internalRequest),
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
