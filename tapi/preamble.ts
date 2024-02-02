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
