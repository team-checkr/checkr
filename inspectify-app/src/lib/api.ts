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
  (
    req: Req,
    options?: ApiOptions
  ): { data: Promise<Res>; abort: () => void } => {
    const controller = new AbortController();
    const promise = fetch(`${getApiBase(options)}${path}`, {
      method,
      headers:
        reqTy == "json" ? { "Content-Type": "application/json" } : void 0,
      body: reqTy == "json" ? JSON.stringify(req) : void 0,
    });
    return {
      data: (async () => {
        const res = await promise;
        if (!res.ok) throw new Error(await res.text());
        if (resTy == "none") return "" as Res;
        if (resTy == "json") return (await res.json()) as Res;
        if (resTy == "text") return (await res.text()) as Res;
        throw new Error(`Unknown response type ${resTy}`);
      })(),
      abort: () => controller.abort(),
    };
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
  <P extends any[], T>(url: (params: P) => string, resTy: ResponseType) =>
  (
    params: P,
    options?: ApiOptions
  ): {
    cancel: () => void;
    listen: (stream: SSEStream<T>) => void;
  } => {
    const source = new EventSource(`${getApiBase(options)}${url(params)}`);

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
    export type Target = string;
    export type TargetKind = "Variable" | "Array";
    export const TARGET_KIND: TargetKind[] = ["Variable", "Array"];
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
    export type Event = { "type": "CompilationStatus", "value": { "status": (inspectify_api.endpoints.CompilationStatus | null) } } | { "type": "JobChanged", "value": { "id": driver.job.JobId, "job": inspectify_api.endpoints.Job } } | { "type": "JobsChanged", "value": { "jobs": driver.job.JobId[] } };
    export type Job = { id: driver.job.JobId, state: driver.job.JobState, kind: driver.job.JobKind, group_name: (string | null), stdout: string, spans: inspectify_api.endpoints.Span[], analysis_data: (inspectify_api.endpoints.AnalysisData | null) }
    export type JobOutput = { output: ce_shell.io.Output, validation: ce_core.ValidationResult }
    export type Target = { name: gcl.ast.Target, kind: gcl.ast.TargetKind }
    export type CompilationStatus = { id: driver.job.JobId, state: driver.job.JobState, error_output: (inspectify_api.endpoints.Span[] | null) }
    export type Span = { text: string, fg: (driver.ansi.Color | null), bg: (driver.ansi.Color | null) }
    export type AnalysisData = { reference_output: ce_shell.io.Output, validation: ce_core.ValidationResult }
  }
}
export const api = {
    generate: request<inspectify_api.endpoints.GenerateParams, ce_shell.io.Input>("json", "POST", "/generate", "json"),
    events: sse<[], inspectify_api.endpoints.Event>(() => `/events`, "json"),
    jobsCancel: request<driver.job.JobId, unknown>("json", "POST", "/jobs/cancel", "none"),
    jobsWait: request<driver.job.JobId, (inspectify_api.endpoints.JobOutput | null)>("json", "POST", "/jobs/wait", "json"),
    analysis: request<ce_shell.io.Input, driver.job.JobId>("json", "POST", "/analysis", "json"),
    gclDot: request<inspectify_api.endpoints.GclDotInput, ce_graph.GraphOutput>("json", "POST", "/gcl/dot", "json"),
    gclFreeVars: request<gcl.ast.Commands, inspectify_api.endpoints.Target[]>("json", "POST", "/gcl/free-vars", "json"),
};
