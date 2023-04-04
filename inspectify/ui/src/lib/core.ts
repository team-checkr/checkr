import type { Analysis, Input, Output } from "./types";

const request = async <T>(
  signal: AbortSignal | null,
  name: string,
  body: unknown
): Promise<T> => {
  const headers = new Headers();
  headers.append("Content-Type", "application/json");

  const req = await fetch(`http://localhost:3000/core/${name}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
    signal,
  });
  return req.json();
};

export const generate_program = async (env: Analysis): Promise<string> =>
  request(null, "generate_program", env);
export const dot = async (
  deterministic: boolean,
  src: string
): Promise<string> => request(null, "dot", [deterministic, src]);
export const complete_input_from_json = async (
  analysis: Analysis,
  input_json: string
): Promise<Input> =>
  request(null, "complete_input_from_json", [analysis, input_json]);
export const generate_input_for = async (
  src: string,
  analysis: Analysis
): Promise<Input | undefined> =>
  request(null, "generate_input_for", [src, analysis]);
export const run_analysis = async (
  src: string,
  input: Input
): Promise<Output | undefined> => request(null, "run_analysis", [src, input]);
