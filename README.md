# Navi Zellij

Navi nods to navigation, Neovim, and “new” in Indian languages.

Use Vim window and tab keys across Neovim splits, Zellij panes, and Zellij tabs. The shell shortcuts work without adding personal Zellij keybinds. Your existing Zellij unlock key and other bindings stay in place.

## Requirements

- Zellij 0.46.0
- Neovim with `vim.system`
- Rust with the `wasm32-wasip1` target to build the plugin

The Zellij WASM plugin handles shell panes. Its Neovim Lua companion uses native window and tab navigation first, then calls Zellij at an edge. Install both for navigation across both environments. The Neovim module also works outside Zellij.

## Install

From this directory:

```sh
rustup target add wasm32-wasip1
cargo build --locked --release --target wasm32-wasip1
install -Dm644 target/wasm32-wasip1/release/navi-zellij.wasm "$HOME/.config/zellij/plugins/navi-zellij.wasm"
install -Dm644 nvim/lua/navi-zellij.lua "$HOME/.config/nvim/lua/navi-zellij.lua"
```

Add the Neovim setup call after your existing mappings:

```lua
require('navi-zellij').setup()
```

Add the plugin URL to `~/.config/zellij/config.kdl`. If you already have a `load_plugins` block, add the URL inside it. Use the absolute path to the installed WASM file:

```kdl
load_plugins {
    "file:/absolute/path/to/.config/zellij/plugins/navi-zellij.wasm"
}
```

Set `default_mode "locked"` in Zellij if you want shell shortcuts available immediately. Keep your own binding for leaving Locked mode. Start a new Zellij session and approve the plugin permissions when prompted.

## Keys

| Focus | Keys | Action |
| --- | --- | --- |
| Neovim | `Ctrl-w h/j/k/l` | Move between Neovim windows; cross into a Zellij pane at an edge |
| Neovim | `gt`, `gT` | Move between Neovim tabs; cross into Zellij tabs at an edge |
| Shell | `Ctrl-w h/j/k/l` | Focus a Zellij pane |
| Shell | `Ctrl-w s/v` | Split below/right |
| Shell | `Ctrl-w c` | Close the focused Zellij pane |
| Shell | `Ctrl-w w/W/p` | Focus the next/previous/last pane |
| Shell | `Ctrl-w +/-/</>` | Grow, shrink, grow right, shrink right |
| Shell | `gt`, `gT` | Next/previous Zellij tab |
| Shell | `Ctrl-v g t`, `Ctrl-v g T` | Type literal `gt` or `gT` |

Other Neovim `Ctrl-w` commands retain their native behavior. In particular, `Ctrl-w c` in Neovim's last window follows Neovim's own behavior. Neovim window and tab movement accepts counts; a numbered `gt` selects that Neovim tab.

## Input behavior

The Zellij plugin intercepts shell keys only in Locked mode with one attached client. It forwards ordinary keys and runs your configured Locked-mode Zellij bindings. It releases input to Neovim and other foreground programs. If Zellij cannot identify the focused command, or if multiple clients attach, shell shortcuts pause until the situation is clear again.

The plugin waits up to 180 ms after a lone `g` or `Ctrl-w` to see whether a shortcut follows. Unrecognized sequences are forwarded in order. At a shell prompt, `gt` and `gT` always switch Zellij tabs; use the literal escape in the table to type them.

## Verify

```sh
cargo test --locked
nvim --headless -u NONE -l tests/nvim_navigation.lua
zellij setup --check
```

The plugin requests permissions to read and change Zellij state, intercept input, send ordinary keys back to the pane, and execute your existing bindings. It does not request permission to reconfigure Zellij.
