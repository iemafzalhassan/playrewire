export function startLab(opts: {
  baseUrl: string;
  loadRuntime: (url: string) => Promise<void>;
  loadBytes: (url: string) => Promise<BufferSource>;
  post: (msg: unknown) => void;
  onMessage: (cb: (msg: any) => void) => void;
}): Promise<void>;
