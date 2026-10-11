#!/bin/sh
# Serves target/book on http://127.0.0.1:PORT (default 8767) without letting the browser cache anything.
set -eu

cd target/book
exec python3 - "${1:-8767}" <<'PY'
import http.server, sys

class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
PY
