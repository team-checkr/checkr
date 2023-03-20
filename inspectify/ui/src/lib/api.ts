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

export const compilationStatus = ({
  signal = null,
}: {
  signal: AbortSignal | null | undefined;
}): Promise<CompilationStatus> =>
  fetch("http://localhost:3000/compilation-status", {
    method: "GET",
    signal,
  }).then((res) => res.json());
