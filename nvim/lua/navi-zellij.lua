--- Vim-style navigation between Neovim windows and Zellij panes or tabs.
--- The module uses Neovim navigation first and calls Zellij only at an edge.
---@module 'navi-zellij'

---@alias NaviZellij.WindowDirection 'h'|'j'|'k'|'l'
---@alias NaviZellij.TabDirection 'next'|'previous'

---@class NaviZellij.Options
---@field run? fun(args: string[]) Override the asynchronous Zellij command runner.
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
---@type fun(): boolean
local in_zellij = function()
    return vim.env.ZELLIJ ~= nil or vim.env.ZELLIJ_SESSION_NAME ~= nil
end

local direction_names = { h = 'left', j = 'down', k = 'up', l = 'right' }

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
    in_zellij = opts.in_zellij or function()
        return vim.env.ZELLIJ ~= nil or vim.env.ZELLIJ_SESSION_NAME ~= nil
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
end

return M
