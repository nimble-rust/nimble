# 🌀 Nimble Protocol

**Nimble Protocol** is a lightweight, deterministic simulation protocol designed for networked games.
It supports connecting clients to a host, downloading game state, and synchronizing predicted
inputs (steps) with authoritative decisions. The protocol ensures that all participants in a game
session stay in sync by sending and receiving the necessary messages. 🕹️

## 🚀 Features

- **Version Support**: Ensures compatibility by checking the deterministic simulation version during connection.
- **Download Game State**: Clients can request a full download of the game state to stay in sync.
- **Participants Management**: Add or remove participants from the game.
- **Step Synchronization**: Send predicted inputs (called steps) and receive authoritative steps from the host.
