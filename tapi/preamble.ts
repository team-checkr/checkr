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
