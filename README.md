# Hangout

Hangout is a fullstack solution written completely in Rust compiled to WASM for one to one private video calling.

It consists of three components:

- Peers run in web browsers
- Signal server for ICE trickling hosted on Cloudflare workers
- Public TURN/STURN server for NAT traversal and relay.

Note: This project is still under active development.

## Why this project is awesome?

The first thing makes this project awesome is the awesome technologies it is powered by. The combination of Rust, WASM and WebRTC.

The second, self-hosted, private, realtime video calling. Meaning everyone can have almost total control over his service.

The third, very easy to deploy.

## Where to host Hangout

Both peer and signal server are compiled to WASM, with the difference that a peer runs in web browser and signal server runs in Cloudflare workers.

The good thing is that anyone who is interested can choose a free plan of Cloudflare workers so that you don't have to pay for any additional cloud severs.

## How to run?

Make sure you have a cloudflare account and read workers documents.

```sh
just build-peer
```

```sh
yarn deploy
```
