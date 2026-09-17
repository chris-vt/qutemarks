# qutemarks

A lightning-fast, local-first bookmark manager designed as a companion for qutebrowser, written in Rust.

`qutemarks` tightly integrates bookmarking directly into your qutebrowser workflow. It acts as a lightweight background daemon and web dashboard, providing a clean UI to organize, tag, and search your saved links.

## Features

- **Seamless Ingestion**: Captures the current page's title and URL directly from qutebrowser via a custom userscript.
- **Responsive Dashboard**: A minimal, fast, multi-column CSS-grid UI for quickly reading and managing saved links, sorted perfectly alphabetically.
- **Start / Landing Page**: A customizable, clean new-tab page (`/start`) where you can pin and organize your most frequently visited URLs.
- **Hidden / Private Tags**: Any tag prefixed with `@` (e.g., `@private`) is strictly hidden from the dashboard and search results by default, and is only revealed when you explicitly select it.
- **Fuzzy Searching**: Instant client-side fuzzy searching across titles, URLs, and notes.
- **Dynamic Tagging**: Filter bookmarks by tags with a single click. Links stripped of all tags are automatically flagged as `untagged` for easy triaging.
- **Data Portability & Backups**: Import HTML bookmarks from Chrome/Firefox, export to CSV/HTML, or use the proprietary `.qutemarks` format for perfect, lossless database backups.
- **Local First**: Powered by an embedded SQLite database. Zero cloud accounts, zero telemetry, full ownership of your data.

## Prerequisites

- **Nix** (Flakes enabled)
- **qutebrowser**

## Installation (NixOS / Home Manager)

Because `qutemarks` is packaged as a Nix flake, you can easily add it to your declarative system configuration.

1. Add the repository to your flake `inputs`:
```nix
inputs = {
  # ... your other inputs
  qutemarks.url = "github:chris-vt/qutemarks"; # Update if your github handle differs
};
```

2. Add the package to your system packages and configure a background user service:
```nix
{ config, pkgs, inputs, ... }: {
  environment.systemPackages = [
    inputs.qutemarks.packages.${pkgs.system}.default
  ];

  # Autolaunch qutemarks in the background
  systemd.user.services.qutemarks = {
    description = "Qutemarks background service for qutebrowser";
    wantedBy = [ "graphical-session.target" ];
    partOf = [ "graphical-session.target" ];
    serviceConfig = {
      ExecStart = "${inputs.qutemarks.packages.${pkgs.system}.default}/bin/qutemarks";
      Restart = "on-failure";
      RestartSec = 5;
    };
  };
}
```

## Setup & Configuration

To get the intended "seamless bookmarking" experience, you must configure qutebrowser to interact with the local server.

### 1. Userscript Setup
Copy the provided ingest script into your qutebrowser userscripts directory and ensure it is executable:

```bash
mkdir -p ~/.local/share/qutebrowser/userscripts
cp scripts/qutebrowser-add.sh ~/.local/share/qutebrowser/userscripts/qutebrowser-add.sh
chmod +x ~/.local/share/qutebrowser/userscripts/qutebrowser-add.sh
```

### 2. Qutebrowser Config & Keybindings
Add the following bindings and settings to your qutebrowser config (e.g., inside `config.py`):

```python
# Use the qutemarks landing page for new empty tabs and startup
c.url.default_page = 'http://localhost:8338/start'
c.url.start_pages = ['http://localhost:8338/start']

# Send current page to qutemarks
config.bind(',b', 'spawn --userscript qutebrowser-add.sh')

# Open the main qutemarks dashboard
config.bind(',B', 'open -t http://localhost:8338')
```

## Usage

1. **Server**: Because of the `systemd` user service configured above, the server binds to `127.0.0.1:8338` in the background when you log in. It automatically initializes its SQLite database on first run.
2. **Save a link**: While browsing in qutebrowser, press `,b`. A native notification will appear confirming the bookmark was saved to the local server.
3. **Organize**: Press `,B` to instantly jump to your local dashboard where you can edit descriptions, assign tags, and search your collection.
4. **Settings & Backups**: Click the **⚙ Settings** button in the bottom left of the dashboard to manage imports, exports, and `.qutemarks` lossless backups.
