build-peer:
    @wasm-pack build -t web -d ../static/pkg peer

file-server: build-peer
    @python3 -m http.server -d static
