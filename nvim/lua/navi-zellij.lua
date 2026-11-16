--- Vim-style navigation and paired-pane jumps between Neovim and Zellij.
--- Window and tab navigation stays inside Neovim until it reaches an edge.
---@module 'navi-zellij'

---@alias NaviZellij.WindowDirection 'h'|'j'|'k'|'l'
---@alias NaviZellij.TabDirection 'next'|'previous'

---@class NaviZellij.CommandResult
---@field code integer
---@field stdout string
---@field stderr string

---@class NaviZellij.Options
---@field run? fun(args: string[]) Override the asynchronous Zellij command runner.
---@field exec? fun(args: string[], callback: fun(result: NaviZellij.CommandResult)) Override the CLI runner used by paired-pane commands.
---@field signal? fun(args: string[]): NaviZellij.CommandResult Override the synchronous editor activity runner.
---@field in_zellij? fun(): boolean Override Zellij session detection.

---@class NaviZellij
local M = {}

--- Run a Zellij command without blocking the Neovim UI.
---@param args string[]
local function default_run(args)
    vim.system(args, {}, function(result)
        if result.code ~= 0 then
            vim.schedule(function()
                vim.notify('Navi Zellij command failed: ' .. (result.stderr or ''), vim.log.levels.WARN)
            end)
        end
    end)
end

---@type fun(args: string[])
local run = default_run

--- Run a Zellij command and return its exit status and output on the UI thread.
---@param args string[]
---@param callback fun(result: NaviZellij.CommandResult)
local function default_exec(args, callback)
    vim.system(args, {}, function(result)
        vim.schedule(function()
            callback(result)
        end)
    end)
end
---@type fun(args: string[], callback: fun(result: NaviZellij.CommandResult))
local exec = default_exec

--- Wait for Zellij to acknowledge an editor activity change.
---@param args string[]
---@return NaviZellij.CommandResult
local function default_signal(args)
    return vim.system(args, { text = true }):wait(1000)
end
---@type fun(args: string[]): NaviZellij.CommandResult
local signal = default_signal

local function default_in_zellij()
    return vim.env.ZELLIJ ~= nil or vim.env.ZELLIJ_SESSION_NAME ~= nil
end
---@type fun(): boolean
local in_zellij = default_in_zellij

-- Fixed-width monotonic start time lets the WASM plugin reject older instances.
local editor_instance = string.format('%020d:%010d', vim.uv.hrtime(), vim.fn.getpid())
local editor_sequence = 0

--- Tell the Zellij plugin whether this Neovim process owns its terminal pane.
---@param action 'enter'|'leave'
local function signal_editor(action)
    local id = tonumber(vim.env.ZELLIJ_PANE_ID)
    if not in_zellij() or not id or id < 0 or id % 1 ~= 0 then
        return
    end
    editor_sequence = editor_sequence + 1
    local payload = string.format('v1 %s %d %s %d', action, id, editor_instance, editor_sequence)
    -- A failed signal is harmless: the plugin also checks the foreground command.
    pcall(signal, { 'zellij', 'pipe', '--name', 'navi-zellij-editor', '--', payload })
end

local direction_names = { h = 'left', j = 'down', k = 'up', l = 'right' }

--- Return the terminal ID of this Neovim pane, or explain why pairing is unavailable.
---@return integer?
local function editor_pane_id()
    local id = tonumber(vim.env.ZELLIJ_PANE_ID)
    if not in_zellij() or not id or id < 0 or id % 1 ~= 0 then
        vim.notify('Navi Zellij: paired panes require a Zellij terminal pane', vim.log.levels.WARN)
        return nil
    end
    return id
end

--- Run a named pipe request and pass its acknowledged response to a callback.
---@param payload string
---@param callback fun(response: string)
local function pipe_request(payload, callback)
    exec({ 'zellij', 'pipe', '--name', 'navi-zellij-pair', '--', payload }, function(result)
        local response = vim.trim(result.stdout or '')
        if result.code ~= 0 or response == '' or response:match('^error:') then
            local detail = response ~= '' and response or vim.trim(result.stderr or '')
            vim.notify(
                'Navi Zellij: pair request failed' .. (detail ~= '' and (': ' .. detail) or ''),
                vim.log.levels.WARN
            )
            return
        end
        callback(response)
    end)
end

--- Choose a live Zellij terminal pane and pair it with this Neovim pane.
---@return nil
function M.pair()
    local editor = editor_pane_id()
    if not editor then
        return
    end
    exec({ 'zellij', 'action', 'list-panes', '--all', '--json' }, function(result)
        if result.code ~= 0 then
            vim.notify('Navi Zellij: could not list panes: ' .. vim.trim(result.stderr or ''), vim.log.levels.WARN)
            return
        end
        local ok, panes = pcall(vim.json.decode, result.stdout or '')
        if not ok or type(panes) ~= 'table' then
            vim.notify('Navi Zellij: invalid pane list from Zellij', vim.log.levels.WARN)
            return
        end
        local choices = {}
        for _, pane in ipairs(panes) do
            if
                type(pane.id) == 'number'
                and pane.id ~= editor
                and not pane.is_plugin
                and not pane.exited
                and not pane.is_held
                and not pane.is_suppressed
                and pane.is_selectable
            then
                table.insert(choices, pane)
            end
        end
        if #choices == 0 then
            vim.notify('Navi Zellij: no available terminal panes to pair', vim.log.levels.INFO)
            return
        end
        vim.ui.select(choices, {
            prompt = 'Pair with Zellij pane:',
            format_item = function(pane)
                return string.format('%s / %s (terminal_%d)', pane.tab_name or '?', pane.title or '?', pane.id)
            end,
        }, function(pane)
            if not pane then
                return
            end
            pipe_request(string.format('v1 pair %d %d', editor, pane.id), function(response)
                if response == 'ok' then
                    vim.notify(string.format('Navi Zellij: Paired with terminal_%d', pane.id), vim.log.levels.INFO)
                else
                    vim.notify('Navi Zellij: unexpected pair response', vim.log.levels.WARN)
                end
            end)
        end)
    end)
end

--- Jump to this Neovim pane's current Zellij partner.
---@return nil
function M.jump()
    local editor = editor_pane_id()
    if not editor then
        return
    end
    pipe_request(string.format('v1 get %d', editor), function(response)
        if response == 'none' then
            vim.notify('Navi Zellij: no pair; use :NaviPair', vim.log.levels.INFO)
            return
        end
        local partner = response:match('^pane (%d+)$')
        if not partner then
            vim.notify('Navi Zellij: unexpected pair response', vim.log.levels.WARN)
            return
        end
        exec({ 'zellij', 'action', 'focus-pane-id', 'terminal_' .. partner }, function(result)
            if result.code ~= 0 then
                vim.notify(
                    'Navi Zellij: could not focus paired pane: ' .. vim.trim(result.stderr or ''),
                    vim.log.levels.WARN
                )
            end
        end)
    end)
end

--- Remove the pair associated with this Neovim pane.
---@return nil
function M.unpair()
    local editor = editor_pane_id()
    if not editor then
        return
    end
    pipe_request(string.format('v1 unpair %d', editor), function(response)
        if response == 'ok' then
            vim.notify('Navi Zellij: Unpaired', vim.log.levels.INFO)
        else
            vim.notify('Navi Zellij: unexpected unpair response', vim.log.levels.WARN)
        end
    end)
end

--- Move through Neovim windows, then across a Zellij pane boundary.
---@param direction NaviZellij.WindowDirection
---@param count? integer Number of window steps; defaults to one.
---@return nil
function M.navigate_window(direction, count)
    count = count or 1
    for _ = 1, count do
        if vim.fn.winnr(direction) ~= vim.fn.winnr() then
            vim.cmd('wincmd ' .. direction)
        elseif in_zellij() then
            run({ 'zellij', 'action', 'move-focus', direction_names[direction] })
            return
        else
            vim.cmd('wincmd ' .. direction)
        end
    end
end

--- Move through Neovim tabs, then across a Zellij tab boundary.
---@param direction NaviZellij.TabDirection
---@param count? integer Number of tab steps; defaults to one.
---@return nil
function M.navigate_tab(direction, count)
    count = count or 1
    for _ = 1, count do
        local current, last = vim.fn.tabpagenr(), vim.fn.tabpagenr('$')
        if direction == 'next' then
            if current < last or not in_zellij() then
                vim.cmd.tabnext()
            else
                run({ 'zellij', 'action', 'go-to-next-tab' })
                return
            end
        elseif current > 1 or not in_zellij() then
            vim.cmd.tabprevious()
        else
            run({ 'zellij', 'action', 'go-to-previous-tab' })
            return
        end
    end
end

--- Install Normal-mode mappings for window and tab navigation.
--- Other `Ctrl-w` commands continue to use Neovim's native behavior.
---@param opts? NaviZellij.Options
---@return nil
function M.setup(opts)
    opts = opts or {}
    run = opts.run or default_run
    exec = opts.exec or default_exec
    signal = opts.signal or default_signal
    in_zellij = opts.in_zellij or default_in_zellij
    local editor_events = vim.api.nvim_create_augroup('NaviZellijEditor', { clear = true })
    vim.api.nvim_create_autocmd('VimEnter', {
        group = editor_events,
        callback = function()
            signal_editor('enter')
        end,
    })
    vim.api.nvim_create_autocmd({ 'VimLeavePre', 'VimSuspend' }, {
        group = editor_events,
        callback = function()
            signal_editor('leave')
        end,
    })
    vim.api.nvim_create_autocmd('VimResume', {
        group = editor_events,
        callback = function()
            signal_editor('enter')
        end,
    })
    if vim.v.vim_did_enter == 1 then
        signal_editor('enter')
    end
    for direction in pairs(direction_names) do
        vim.keymap.set('n', '<C-w>' .. direction, function()
            M.navigate_window(direction, vim.v.count1)
        end, { silent = true, desc = 'Move to Vim window or Zellij pane' })
    end
    vim.keymap.set('n', 'gt', function()
        if vim.v.count > 0 then
            vim.cmd.tabnext(vim.v.count)
        else
            M.navigate_tab('next')
        end
    end, { silent = true, desc = 'Next Vim or Zellij tab' })
    vim.keymap.set('n', 'gT', function()
        M.navigate_tab('previous', vim.v.count1)
    end, { silent = true, desc = 'Previous Vim or Zellij tab' })
    vim.api.nvim_create_user_command(
        'NaviPair',
        M.pair,
        { desc = 'Pair this Neovim pane with a Zellij pane', force = true }
    )
    vim.api.nvim_create_user_command('NaviJump', M.jump, { desc = 'Jump to the paired Zellij pane', force = true })
    vim.api.nvim_create_user_command('NaviUnpair', M.unpair, { desc = 'Remove this Neovim pane pair', force = true })
end

return M
