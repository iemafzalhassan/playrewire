export function startLab(opts: {
  baseUrl: string;
  loadText: (url: string) => Promise<string>;
  loadBytes: (url: string) => Promise<BufferSource>;
  post: (msg: unknown) => void;
  onMessage: (cb: (msg: any) => void) => void;
}): Promise<void>;
