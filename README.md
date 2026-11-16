# Navi Zellij

Use familiar Vim keys to move between Neovim windows, Zellij panes, and Zellij tabs. **Navi** evokes navigation, Neovim, and “new” in several Indian languages.

Navi Zellij has two parts:

| Part | What it does |
| --- | --- |
| Zellij WASM plugin | Handles Vim-style shortcuts at a shell prompt. It uses your existing Locked-mode bindings and needs no new personal Zellij keybinds. |
| Neovim Lua module | Moves within Neovim first. At an outer window or tab edge, it asks Zellij to move instead. |

Install both parts for navigation in Neovim and shell panes. The Neovim module also works when Zellij is not running.

## Requirements

- Zellij 0.46.0
- Neovim with `vim.system`
- Rust and the `wasm32-wasip1` target to build the Zellij plugin

Supported shells are `sh`, `bash`, `zsh`, `fish`, `nu`, `dash`, and `ksh`.

## Install

Build the WASM plugin and copy both parts from this repository:

```sh
rustup target add wasm32-wasip1
cargo build --locked --release --target wasm32-wasip1
mkdir -p "$HOME/.config/zellij/plugins" "$HOME/.config/nvim/lua"
cp target/wasm32-wasip1/release/navi-zellij.wasm "$HOME/.config/zellij/plugins/navi-zellij.wasm"
cp nvim/lua/navi-zellij.lua "$HOME/.config/nvim/lua/navi-zellij.lua"
```

Load the Neovim module **after** your other mappings:

```lua
require('navi-zellij').setup()
```

Add the WASM file to `~/.config/zellij/config.kdl`. Replace the example with its **absolute path**. If you already have a `load_plugins` block, add the file URL inside that block.

```kdl
load_plugins {
    "file:/home/you/.config/zellij/plugins/navi-zellij.wasm"
}
```

For shell shortcuts to be available immediately, set `default_mode "locked"` in Zellij. Keep your own binding for leaving Locked mode. Start a new Zellij session and grant the plugin permissions when prompted.

## Keys

| Where | Keys | Result |
| --- | --- | --- |
| Neovim | `Ctrl-w h/j/k/l` | Move between Neovim windows, then into a Zellij pane at an edge |
| Neovim | `gt`, `gT` | Move between Neovim tabs, then into a Zellij tab at an edge |
| Shell | `Ctrl-w h/j/k/l` | Focus the Zellij pane in that direction |
| Shell | `Ctrl-w s/v` | Split below/right |
| Shell | `Ctrl-w c` | Close the focused Zellij pane |
| Shell | `Ctrl-w w/W/p` | Focus the next/previous/last pane |
| Shell | `Ctrl-w +/-/</>` | Resize the focused pane |
| Shell | `gt`, `gT` | Go to the next/previous Zellij tab |
| Shell | `Ctrl-v g t`, `Ctrl-v g T` | Type `gt` or `gT` literally |
| Shell | `Ctrl-v Ctrl-v` | Send one native `Ctrl-v` to the shell |

The first `Ctrl-v` is Navi Zellij's literal-key escape while shell shortcuts are active. Press it twice for the shell's own `Ctrl-v` behavior, such as paste in fish.

Only the listed Neovim mappings are added. Other `Ctrl-w` commands keep their native Neovim behavior; in particular, `Ctrl-w c` in Neovim's last window follows Neovim's safe refusal. Counts work within Neovim; crossing a Zellij edge performs one Zellij move. A numbered `gt` selects that Neovim tab.

## When shell shortcuts are active

The Zellij plugin handles input only in Locked mode, with one attached client focused on a recognized shell. Your existing Locked-mode bindings take priority. When Zellij identifies Neovim or another foreground program, interception stops. If no command can be resolved, shell shortcuts pause.

Zellij may report the pane’s shell when it cannot inspect a foreground program. In that case, Navi Zellij can still interpret shell shortcuts while that program is running.

Navi Zellij waits up to 180 ms after `g` or `Ctrl-w` for a second key. A lone prefix or an unrecognized sequence is sent to the shell. At a shell prompt, `gt` and `gT` switch Zellij tabs, so use the literal escape above when you need to type those characters.

The plugin requests permission to read and change Zellij state, intercept keys, send keys to the pane, and run your existing bindings. It does not change your Zellij configuration.

## Verify

```sh
cargo test --locked
nvim --headless -u NONE -i NONE -l tests/nvim_navigation.lua
zellij setup --check
```

## License

[MIT](LICENSE).
