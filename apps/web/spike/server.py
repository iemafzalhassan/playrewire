"""Static server for the Task 2 spike: serves apps/web, logs GET /__fps lines, and
saves POST /__shot bodies (an in-page PNG capture of the canvas) to the path given
by --shot. Usage: python3 spike/server.py PORT [--shot out.png]"""
import http.server
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SHOT = sys.argv[sys.argv.index("--shot") + 1] if "--shot" in sys.argv else None


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=str(ROOT), **kw)

    def do_GET(self):
        if self.path.startswith("/__fps"):
            self.log_message("%s", self.path)
            self.send_response(204)
            self.end_headers()
            return
        super().do_GET()

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if self.path.startswith("/__shot") and SHOT:
            Path(SHOT).write_bytes(body)
            self.log_message("saved shot %s (%d bytes)", SHOT, len(body))
        self.send_response(204)
        self.end_headers()


http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
