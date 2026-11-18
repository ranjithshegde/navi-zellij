# Navi Zellij

Use familiar Vim keys to move between Neovim windows, Zellij panes, and Zellij tabs. **Navi** evokes navigation, Neovim, and “new” in several Indian languages.

Navi Zellij has two parts:

| Part | What it does |
| --- | --- |
| Zellij WASM plugin | Handles Vim-style shortcuts in shells and terminal apps, and remembers pane pairs. It uses your existing Locked-mode bindings and needs no new personal Zellij keybinds. |
| Neovim Lua module | Moves within Neovim first, asks Zellij to move at an outer edge, and provides commands to pair and jump between panes. |

Install both parts for navigation in Neovim, shells, and other terminal apps. The Neovim module also works when Zellij is not running.

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

Add the WASM file to `~/.config/zellij/config.kdl`. Zellij expands `~` to your home directory in a local plugin URL. If you already have a `load_plugins` block, add the file URL inside that block.

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/navi-zellij.wasm"
}
```

For terminal shortcuts to be available immediately, set `default_mode "locked"` in Zellij. Keep your own binding for leaving Locked mode. Start a new Zellij session and grant the plugin permissions when prompted.

## Keys

| Where | Keys | Result |
| --- | --- | --- |
| Neovim | `Ctrl-w h/j/k/l` | Move between Neovim windows, then into a Zellij pane at an edge |
| Neovim | `gt`, `gT` | Move between Neovim tabs, then into a Zellij tab at an edge |
| Shell or terminal app | `Ctrl-w h/j/k/l` | Focus the Zellij pane in that direction |
| Shell or terminal app | `Ctrl-w s/v` | Split below/right |
| Shell or terminal app | `Ctrl-w c` | Close the focused Zellij pane |
| Shell or terminal app | `Ctrl-w w/W` | Focus the next/previous pane |
| Shell or terminal app | `Ctrl-w p` | Jump to the paired editor, or focus the last pane when unpaired |
| Shell or terminal app | `Ctrl-w +/-/</>` | Resize the focused pane |
| Shell | `gt`, `gT` | Go to the next/previous Zellij tab |
| Shell | `g`, pause, then `t` or `T` | Type `gt` or `gT` literally |
| Terminal app | `gt`, `gT` | Go straight to the app |
| Shell or terminal app | `Ctrl-v` | Go straight to the program, including Claude Code's image paste |

At a shell prompt, pause for at least 180 ms after `g` before typing `t` or `T` to enter those characters literally. `Ctrl-v` is always sent to the program.

Only the listed Neovim mappings are added. Other `Ctrl-w` commands keep their native Neovim behavior; in particular, `Ctrl-w c` in Neovim's last window follows Neovim's safe refusal. Counts work within Neovim; crossing a Zellij edge performs one Zellij move. A numbered `gt` selects that Neovim tab.

## Paired panes

Pair your editor with a terminal pane for tests, logs, or an agent:

1. In Neovim, run `:NaviPair` and choose a live terminal pane. The picker shows its tab, title, and pane ID.
2. Run `:NaviJump` to focus that pane, even if it is on another Zellij tab.
3. In the paired shell or terminal app, press `Ctrl-w p` to return to the editor. This works after you visit other panes. In an unpaired pane, `Ctrl-w p` still focuses the last pane.

Run `:NaviUnpair` in Neovim to remove the pair. Choosing a pane that is already paired replaces its old pair; the previous editor will need to pair again.

Pairs belong to Zellij terminal panes and last while the WASM plugin stays loaded. Restarting the plugin or session clears them. No additional keybindings or shell aliases are required.

The commands need both installed parts and Zellij's `ZELLIJ_PANE_ID` environment variable. When upgrading, Zellij may ask you to grant the plugin its new CLI pipe permission.

## When terminal shortcuts are active

The Zellij plugin handles input only in Locked mode, with one attached client focused on an identifiable terminal program. Your existing Locked-mode bindings take priority. The Neovim companion tells the plugin when Neovim is active, so its native window commands stay in Neovim. If the foreground command cannot be resolved, interception pauses.

If Zellij reports a shell even after checking the foreground command, Navi Zellij can apply shell shortcuts inside another app. This depends on what Zellij can detect on your system.

Navi Zellij waits up to 180 ms after `Ctrl-w` for a second key, and after `g` at a shell prompt. A lone prefix or an unrecognized sequence is sent to the program. This lets a lone `Ctrl-w` retain an app's native action, such as deleting a word in Claude Code, after a short delay. At a shell prompt, `gt` and `gT` switch Zellij tabs when typed within that interval.

The plugin requests permission to read and change Zellij state, intercept keys, send keys to the pane, run your existing bindings, and handle CLI pipes for paired panes and Neovim activity. It does not change your Zellij configuration.

## Verify

```sh
cargo test --locked
nvim --headless -u NONE -i NONE -l tests/nvim_navigation.lua
nvim --headless -u NONE -i NONE -l tests/nvim_pairs.lua
nvim --headless -u NONE -i NONE -l tests/nvim_editor.lua
zellij setup --check
```

## License

[MIT](LICENSE).
