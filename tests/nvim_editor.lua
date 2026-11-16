vim.opt.runtimepath:prepend(vim.fn.getcwd() .. '/nvim')
local nav = require('navi-zellij')
local old_pane, old_zellij = vim.env.ZELLIJ_PANE_ID, vim.env.ZELLIJ
vim.env.ZELLIJ_PANE_ID = '0'
vim.env.ZELLIJ = '1'

local calls = {}
local function signal(args)
    table.insert(calls, args)
    return { code = 0, stdout = 'ok\n', stderr = '' }
end

local function trigger(event)
    vim.api.nvim_exec_autocmds(event, { pattern = '*' })
end

local function message(index, expected_action)
    local args = calls[index]
    assert(args, 'missing editor activity signal ' .. index)
    assert(table.concat(vim.list_slice(args, 1, 5), ' ') == 'zellij pipe --name navi-zellij-editor --')
    local action, pane, instance, sequence = args[6]:match('^v1 (%w+) (%d+) ([^ ]+) (%d+)$')
    assert(action == expected_action and pane == '0', 'wrong editor activity signal')
    local started, pid = instance:match('^(%d+):(%d+)$')
    assert(
        started and #started == 20 and pid and #pid == 10 and sequence,
        'signal needs an ordered instance and sequence'
    )
    return instance, tonumber(sequence)
end

nav.setup({
    in_zellij = function()
        return true
    end,
    signal = signal,
})
calls = {}
trigger('VimEnter')
trigger('VimSuspend')
trigger('VimResume')
trigger('VimLeavePre')
local instance, first = message(1, 'enter')
for index, action in ipairs({ 'leave', 'enter', 'leave' }) do
    local current_instance, sequence = message(index + 1, action)
    assert(current_instance == instance and sequence > first, 'signals must be ordered per editor')
    first = sequence
end

nav.setup({
    in_zellij = function()
        return true
    end,
    signal = signal,
})
calls = {}
trigger('VimResume')
assert(#calls == 1, 'repeated setup must not duplicate lifecycle autocmds')
message(1, 'enter')

nav.setup({
    in_zellij = function()
        return false
    end,
    signal = signal,
})
calls = {}
trigger('VimEnter')
trigger('VimLeavePre')
assert(#calls == 0, 'Neovim outside Zellij must not send activity signals')

vim.env.ZELLIJ_PANE_ID, vim.env.ZELLIJ = old_pane, old_zellij
print('Neovim editor activity tests passed')
