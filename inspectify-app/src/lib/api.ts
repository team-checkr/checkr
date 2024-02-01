export type ApiOptions = {
  fetch?: typeof fetch;
  apiBase?: string;
  headers?: Record<string, string>;
};

let GLOBAL_API_BASE = "";
export const getApiBase = (options?: ApiOptions) =>
  options?.apiBase ?? GLOBAL_API_BASE;
export const setGlobalApiBase = (apiBase: string) =>
  (GLOBAL_API_BASE = apiBase);

type RequestType = "none" | "json";
type ResponseType = "none" | "text" | "json";
type Method = "DELETE" | "GET" | "PUT" | "POST" | "HEAD" | "TRACE" | "PATCH";
const request =
  <Req, Res>(
    reqTy: RequestType,
    method: Method,
    path: string,
    resTy: ResponseType
  ) =>
  async (req: Req, options?: ApiOptions): Promise<Res> => {
    const res = await fetch(`${getApiBase(options)}${path}`, {
      method,
      headers:
        reqTy == "json" ? { "Content-Type": "application/json" } : void 0,
      body: reqTy == "json" ? JSON.stringify(req) : void 0,
    });
    if (!res.ok) {
      throw new Error(await res.text());
    }
    if (resTy == "none") {
      return "" as Res;
    }
    if (resTy == "json") {
      return (await res.json()) as Res;
    }
    if (resTy == "text") {
      return (await res.text()) as Res;
    }
    throw new Error(`Unknown response type ${resTy}`);
  };

export type SSEStream<T> = (
  event:
    | { type: "message"; data: T }
    | {
        type: "error";
        event: Event;
      }
) => void;

const sse =
  <T>(url: string, resTy: ResponseType) =>
  (
    options?: ApiOptions
  ): {
    cancel: () => void;
    listen: (stream: SSEStream<T>) => void;
  } => {
    const source = new EventSource(`${getApiBase(options)}${url}`);

    let stream: SSEStream<T> | null = null;

    source.onmessage = (event) => {
      const data = event.data;
      if (resTy == "text") {
        stream?.({ type: "message", data });
      } else if (resTy == "json") {
        stream?.({ type: "message", data: JSON.parse(data) });
      } else {
        throw new Error(`Unknown response type: ${resTy}`);
      }
    };
    source.onerror = (event) => {
      stream?.({ type: "error", event });
    };
    return {
      cancel: () => source.close(),
      listen: (newStream) => (stream = newStream),
    };
  };
export namespace ce_core {
  export type ValidationResult = { "type": "CorrectTerminated" } | { "type": "CorrectNonTerminated", "iterations": number } | { "type": "Mismatch", "reason": string } | { "type": "TimeOut" };
}
export namespace ce_graph {
  export type GraphOutput = { dot: string }
  export type GraphInput = { commands: gcl.ast.Commands, determinism: gcl.pg.Determinism }
}
export namespace ce_parse {
  export type ParseInput = { commands: gcl.ast.Commands }
  export type ParseOutput = { pretty: string }
}
export namespace ce_shell {
  export type Envs = { "analysis": "Parse", "io": { "input": ce_parse.ParseInput, "output": ce_parse.ParseOutput } } | { "analysis": "Graph", "io": { "input": ce_graph.GraphInput, "output": ce_graph.GraphOutput } } | { "analysis": "Sign", "io": { "input": ce_sign.SignInput, "output": ce_sign.SignOutput } };
  export type Analysis = "Parse" | "Graph" | "Sign";
  export const ANALYSIS: Analysis[] = ["Parse", "Graph", "Sign"];
  export namespace io {
    export type Input = { analysis: ce_shell.Analysis, json: unknown }
    export type Output = { analysis: ce_shell.Analysis, json: unknown }
  }
}
export namespace ce_sign {
  export type SignInput = { commands: gcl.ast.Commands, determinism: gcl.pg.Determinism, assignment: gcl.memory.Memory }
  export type SignOutput = { initial_node: string, final_node: string, nodes: Record<string, gcl.memory.Memory[]> }
  export namespace semantics {
    export type Sign = { "Case": "Positive" } | { "Case": "Zero" } | { "Case": "Negative" };
    export const SIGN: Sign[] = [{ "Case": "Positive" }, { "Case": "Zero" }, { "Case": "Negative" }];
    export type Signs = ce_sign.semantics.Sign[];
  }
}
export namespace driver {
  export namespace ansi {
    export type Color = "Black" | "Red" | "Green" | "Yellow" | "Blue" | "Magenta" | "Cyan" | "White" | "Default" | "BrightBlack" | "BrightRed" | "BrightGreen" | "BrightYellow" | "BrightBlue" | "BrightMagenta" | "BrightCyan" | "BrightWhite";
    export const COLOR: Color[] = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White", "Default", "BrightBlack", "BrightRed", "BrightGreen", "BrightYellow", "BrightBlue", "BrightMagenta", "BrightCyan", "BrightWhite"];
  }
  export namespace job {
    export type JobId = number;
    export type JobState = "Queued" | "Running" | "Succeeded" | "Canceled" | "Failed" | "Warning";
    export const JOB_STATE: JobState[] = ["Queued", "Running", "Succeeded", "Canceled", "Failed", "Warning"];
    export type JobKind = { "kind": "Compilation", "data": {  } } | { "kind": "Analysis", "data": [ce_shell.Analysis, ce_shell.io.Input] };
  }
}
export namespace gcl {
  export namespace ast {
    export type Commands = string;
    export type Variable = string;
    export type Array = string;
  }
  export namespace memory {
    export type Memory = { variables: Record<gcl.ast.Variable, ce_sign.semantics.Sign>, arrays: Record<gcl.ast.Array, ce_sign.semantics.Signs> }
  }
  export namespace pg {
    export type Determinism = { "Case": "Deterministic" } | { "Case": "NonDeterministic" };
    export const DETERMINISM: Determinism[] = [{ "Case": "Deterministic" }, { "Case": "NonDeterministic" }];
  }
}
export namespace inspectify_api {
  export namespace endpoints {
    export type GenerateParams = { analysis: ce_shell.Analysis }
    export type GclDotInput = { determinism: gcl.pg.Determinism, commands: gcl.ast.Commands }
    export type CompilationStatus = { id: driver.job.JobId, state: driver.job.JobState, error_output: (inspectify_api.endpoints.Span[] | null) }
    export type JobOutput = { output: ce_shell.io.Output, validation: ce_core.ValidationResult }
    export type Job = { id: driver.job.JobId, state: driver.job.JobState, kind: driver.job.JobKind, stdout: string, spans: inspectify_api.endpoints.Span[] }
    export type Span = { text: string, fg: (driver.ansi.Color | null), bg: (driver.ansi.Color | null) }
  }
}
export const api = {
    generate: request<inspectify_api.endpoints.GenerateParams, ce_shell.io.Input>("json", "POST", "/generate", "json"),
    jobs: sse<inspectify_api.endpoints.Job[]>("/jobs", "json"),
    analysis: request<ce_shell.io.Input, driver.job.JobId>("json", "POST", "/analysis", "json"),
    cancelJob: request<driver.job.JobId, unknown>("json", "POST", "/cancel-job", "none"),
    waitForJob: request<driver.job.JobId, (inspectify_api.endpoints.JobOutput | null)>("json", "POST", "/wait-for-job", "json"),
    gclDot: request<inspectify_api.endpoints.GclDotInput, ce_graph.GraphOutput>("json", "POST", "/gcl-dot", "json"),
    compilationStatus: sse<(inspectify_api.endpoints.CompilationStatus | null)>("/compilation-status", "json"),
};
