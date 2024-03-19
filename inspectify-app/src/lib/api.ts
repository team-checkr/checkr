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

export namespace Calculator {
  export type Input = {
    "expression": string
  };
  export type Output = {
    "result": string,
    "error": string
  };
}
export namespace Compiler {
  export type Input = {
    "commands": string,
    "determinism": GCL.Determinism
  };
  export type Output = {
    "dot": string
  };
}
export namespace GCL {
  export type Determinism =
    | "Deterministic"
    | "NonDeterministic";
  export const DETERMINISM: Determinism[] = ["Deterministic", "NonDeterministic"];
  export type TargetDef = {
    "name": string,
    "kind": GCL.TargetKind
  };
  export type TargetKind =
    | "Variable"
    | "Array";
  export const TARGET_KIND: TargetKind[] = ["Variable", "Array"];
  export type Variable = string;
  export type Array = string;
}
export namespace Interpreter {
  export type Input = {
    "commands": string,
    "determinism": GCL.Determinism,
    "assignment": Interpreter.InterpreterMemory,
    "trace_length": number
  };
  export type Output = {
    "initial_node": string,
    "final_node": string,
    "dot": string,
    "trace": Interpreter.Step[],
    "termination": Interpreter.TerminationState
  };
  export type InterpreterMemory = {
    "variables": Record<GCL.Variable, number>,
    "arrays": Record<GCL.Array, number[]>
  };
  export type TerminationState =
    | "Running"
    | "Stuck"
    | "Terminated";
  export const TERMINATION_STATE: TerminationState[] = ["Running", "Stuck", "Terminated"];
  export type Step = {
    "action": string,
    "node": string,
    "memory": Interpreter.InterpreterMemory
  };
}
export namespace Parser {
  export type Input = {
    "commands": string
  };
  export type Output = {
    "pretty": string
  };
}
export namespace SecurityAnalysis {
  export type Input = {
    "commands": string,
    "classification": Record<string, string>,
    "lattice": SecurityAnalysis.SecurityLatticeInput
  };
  export type Output = {
    "actual": SecurityAnalysis.Flow[],
    "allowed": SecurityAnalysis.Flow[],
    "violations": SecurityAnalysis.Flow[],
    "is_secure": boolean
  };
  export type Meta = {
    "lattice": SecurityAnalysis.SecurityLattice,
    "targets": GCL.TargetDef[]
  };
  export type SecurityLatticeInput = {
    "rules": SecurityAnalysis.Flow[]
  };
  export type SecurityLattice = {
    "allowed": SecurityAnalysis.Flow[]
  };
  export type Flow = {
    "from": string,
    "into": string
  };
}
export namespace SignAnalysis {
  export type Input = {
    "commands": string,
    "determinism": GCL.Determinism,
    "assignment": SignAnalysis.SignMemory
  };
  export type Output = {
    "initial_node": string,
    "final_node": string,
    "nodes": Record<string, SignAnalysis.SignMemory[]>,
    "dot": string
  };
  export type SignMemory = {
    "variables": Record<GCL.Variable, SignAnalysis.Sign>,
    "arrays": Record<GCL.Array, SignAnalysis.Sign[]>
  };
  export type Sign =
    | "Positive"
    | "Zero"
    | "Negative";
  export const SIGN: Sign[] = ["Positive", "Zero", "Negative"];
}
export namespace ce_core {
  export type ValidationResult =
    | { "type": "CorrectTerminated" }
    | { "type": "CorrectNonTerminated", "iterations": number }
    | { "type": "Mismatch", "reason": string }
    | { "type": "TimeOut" };
}
export namespace ce_shell {
  export type Envs =
    | { "analysis": "Calculator", "io": { "input": Calculator.Input, "output": Calculator.Output, "meta": void } }
    | { "analysis": "Parser", "io": { "input": Parser.Input, "output": Parser.Output, "meta": void } }
    | { "analysis": "Compiler", "io": { "input": Compiler.Input, "output": Compiler.Output, "meta": void } }
    | { "analysis": "Interpreter", "io": { "input": Interpreter.Input, "output": Interpreter.Output, "meta": GCL.TargetDef[] } }
    | { "analysis": "Sign", "io": { "input": SignAnalysis.Input, "output": SignAnalysis.Output, "meta": GCL.TargetDef[] } }
    | { "analysis": "Security", "io": { "input": SecurityAnalysis.Input, "output": SecurityAnalysis.Output, "meta": SecurityAnalysis.Meta } };
  export type Analysis =
    | "Calculator"
    | "Parser"
    | "Compiler"
    | "Interpreter"
    | "Sign"
    | "Security";
  export const ANALYSIS: Analysis[] = ["Calculator", "Parser", "Compiler", "Interpreter", "Sign", "Security"];
  export namespace io {
    export type Input = {
      "analysis": ce_shell.Analysis,
      "json": any,
      "hash": ce_shell.io.Hash
    };
    export type Hash = {
      "bytes": number[]
    };
    export type Meta = {
      "analysis": ce_shell.Analysis,
      "json": any
    };
    export type Output = {
      "analysis": ce_shell.Analysis,
      "json": any,
      "hash": ce_shell.io.Hash
    };
  }
}
export namespace driver {
  export namespace ansi {
    export type Color =
      | "Black"
      | "Red"
      | "Green"
      | "Yellow"
      | "Blue"
      | "Magenta"
      | "Cyan"
      | "White"
      | "Default"
      | "BrightBlack"
      | "BrightRed"
      | "BrightGreen"
      | "BrightYellow"
      | "BrightBlue"
      | "BrightMagenta"
      | "BrightCyan"
      | "BrightWhite";
    export const COLOR: Color[] = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White", "Default", "BrightBlack", "BrightRed", "BrightGreen", "BrightYellow", "BrightBlue", "BrightMagenta", "BrightCyan", "BrightWhite"];
  }
  export namespace job {
    export type JobId = number;
    export type JobState =
      | "Queued"
      | "Running"
      | "Succeeded"
      | "Canceled"
      | "Failed"
      | "Warning"
      | "Timeout"
      | "OutputLimitExceeded";
    export const JOB_STATE: JobState[] = ["Queued", "Running", "Succeeded", "Canceled", "Failed", "Warning", "Timeout", "OutputLimitExceeded"];
    export type JobKind =
      | { "kind": "Compilation" }
      | { "kind": "Analysis", "data": ce_shell.io.Input };
  }
}
export namespace inspectify {
  export namespace checko {
    export type GroupStatus =
      | "Initial"
      | "CheckingForUpdate"
      | "Compiling"
      | "Testing"
      | "CompilationError"
      | "Finished";
    export const GROUP_STATUS: GroupStatus[] = ["Initial", "CheckingForUpdate", "Compiling", "Testing", "CompilationError", "Finished"];
    export namespace config {
      export type GroupsConfig = {
        "ignored_authors": string[],
        "groups": inspectify.checko.config.GroupConfig[]
      };
      export type GroupName = string;
      export type GroupConfig = {
        "name": inspectify.checko.config.GroupName,
        "git": (string | null),
        "path": (string | null),
        "run": (string | null)
      };
    }
    export namespace scoreboard {
      export type PublicState = {
        "last_finished": (string | null),
        "analysis": inspectify.checko.scoreboard.PublicAnalysis[],
        "groups": inspectify.checko.scoreboard.PublicGroup[]
      };
      export type PublicAnalysis = {
        "analysis": ce_shell.Analysis,
        "programs": (ce_shell.io.Input | null)[]
      };
      export type PublicGroup = {
        "name": inspectify.checko.config.GroupName,
        "analysis_results": inspectify.checko.scoreboard.PublicAnalysisResults[]
      };
      export type PublicAnalysisResults = {
        "analysis": ce_shell.Analysis,
        "status": inspectify.checko.GroupStatus,
        "last_hash": (string | null),
        "results": inspectify.checko.scoreboard.PublicProgramResult[]
      };
      export type PublicProgramResult = {
        "state": driver.job.JobState
      };
    }
  }
  export namespace endpoints {
    export type ReferenceExecution = {
      "meta": ce_shell.io.Meta,
      "output": (ce_shell.io.Output | null),
      "error": (string | null)
    };
    export type PublicEvent =
      | { "type": "Reset" }
      | { "type": "StateChanged", "value": inspectify.checko.scoreboard.PublicState };
    export type GenerateParams = {
      "analysis": ce_shell.Analysis
    };
    export type Event =
      | { "type": "Reset" }
      | { "type": "CompilationStatus", "value": { "status": (inspectify.endpoints.CompilationStatus | null) } }
      | { "type": "JobChanged", "value": { "job": inspectify.endpoints.Job } }
      | { "type": "JobsChanged", "value": { "jobs": driver.job.JobId[] } }
      | { "type": "GroupsConfig", "value": { "config": inspectify.checko.config.GroupsConfig } }
      | { "type": "ProgramsConfig", "value": { "programs": inspectify.endpoints.Program[] } };
    export type Job = {
      "id": driver.job.JobId,
      "state": driver.job.JobState,
      "kind": driver.job.JobKind,
      "group_name": (inspectify.checko.config.GroupName | null),
      "stdout": string,
      "spans": inspectify.endpoints.Span[],
      "analysis_data": (inspectify.endpoints.AnalysisData | null)
    };
    export type AnalysisExecution = {
      "id": driver.job.JobId
    };
    export type CompilationStatus = {
      "id": driver.job.JobId,
      "state": driver.job.JobState,
      "error_output": (inspectify.endpoints.Span[] | null)
    };
    export type Program = {
      "hash": ce_shell.io.Hash,
      "hash_str": string,
      "input": ce_shell.io.Input
    };
    export type Span = {
      "text": string,
      "fg": (driver.ansi.Color | null),
      "bg": (driver.ansi.Color | null)
    };
    export type AnalysisData = {
      "meta": ce_shell.io.Meta,
      "output": (ce_shell.io.Output | null),
      "reference_output": (ce_shell.io.Output | null),
      "validation": (ce_core.ValidationResult | null)
    };
  }
}
export const api = {
    generate: request<inspectify.endpoints.GenerateParams, ce_shell.io.Input>("json", "POST", "/generate", "json"),
    events: sse<[], inspectify.endpoints.Event>(() => `/events`, "json"),
    checkoPublic: sse<[], inspectify.endpoints.PublicEvent>(() => `/checko-public`, "json"),
    jobsCancel: request<driver.job.JobId, void>("json", "POST", "/jobs/cancel", "none"),
    analysis: request<ce_shell.io.Input, (inspectify.endpoints.AnalysisExecution | null)>("json", "POST", "/analysis", "json"),
    reference: request<ce_shell.io.Input, inspectify.endpoints.ReferenceExecution>("json", "POST", "/reference", "json"),
};
