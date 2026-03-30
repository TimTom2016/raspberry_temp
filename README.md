# raspberry_temp

A lightweight temperature monitoring system for Raspberry Pi. Reads SoC temperature via the official `vcgencmd measure_temp` command and stores readings in SQLite with a simple web dashboard for real-time updates.

## Quick Start

```bash
cargo run
```

The server binds to `0.0.0.0:3000` by default. Access the dashboard at `http://your-pi:3000`.

## Configuration

| Variable     | Default           | Description             |
| ------------ | ----------------- | ----------------------- |
| `DATABASE_URL` | `sqlite://temps.db` | Path to SQLite database |
| `BIND_ADDR`    | `0.0.0.0:3000`      | Address to listen on    |

## Features

- Automatic temperature readings via vcgencmd (SoC sensor)
- Web dashboard with live updates via HTMX
- SQLite storage with SeaORM
- Background task for compressing old readings
- Graceful shutdown on SIGINT/SIGTERM
