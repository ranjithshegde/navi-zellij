vim.opt.runtimepath:prepend(vim.fn.getcwd() .. '/nvim')
local nav = require('navi-zellij')
local calls, responses, choices, notices = {}, {}, {}, {}
local old_select, old_notify = vim.ui.select, vim.notify
local old_pane = vim.env.ZELLIJ_PANE_ID
vim.env.ZELLIJ_PANE_ID = '0'

vim.notify = function(message)
    table.insert(notices, message)
end
vim.ui.select = function(items, opts, callback)
    choices = items
    assert(opts.format_item(items[1]) == 'code / tests (terminal_12)', 'picker label must identify the pane')
    callback(items[1])
end
local function enqueue(stdout, code)
    table.insert(responses, { stdout = stdout, code = code or 0, stderr = '' })
end
nav.setup({
    in_zellij = function()
        return true
    end,
    exec = function(args, callback)
        table.insert(calls, args)
        callback(table.remove(responses, 1) or error('unexpected command'))
    end,
})

local panes = {
    {
        id = 0,
        is_plugin = false,
        title = 'editor',
        tab_name = 'code',
        exited = false,
        is_suppressed = false,
        is_selectable = true,
    },
    {
        id = 12,
        is_plugin = false,
        title = 'tests',
        tab_name = 'code',
        exited = false,
        is_suppressed = false,
        is_selectable = true,
    },
    {
        id = 13,
        is_plugin = false,
        title = 'status',
        tab_name = 'code',
        exited = false,
        is_suppressed = false,
        is_selectable = false,
    },
    {
        id = 14,
        is_plugin = true,
        title = 'plugin',
        tab_name = 'code',
        exited = false,
        is_suppressed = false,
        is_selectable = true,
    },
    {
        id = 15,
        is_plugin = false,
        title = 'old',
        tab_name = 'logs',
        exited = true,
        is_suppressed = false,
        is_selectable = true,
    },
    {
        id = 16,
        is_plugin = false,
        title = 'hidden',
        tab_name = 'logs',
        exited = false,
        is_suppressed = true,
        is_selectable = true,
    },
    {
        id = 17,
        is_plugin = false,
        title = 'held',
        tab_name = 'logs',
        exited = false,
        is_held = true,
        is_suppressed = false,
        is_selectable = true,
    },
}
enqueue(vim.json.encode(panes))
enqueue('ok\n')
nav.pair()
assert(#choices == 1 and choices[1].id == 12, 'picker must offer only live selectable partner panes')
assert(table.concat(calls[1], ' ') == 'zellij action list-panes --all --json')
assert(table.concat(calls[2], ' ') == 'zellij pipe --name navi-zellij-pair -- v1 pair 0 12')
assert(notices[1] and notices[1]:find('Paired', 1, true), 'successful pairing should be confirmed')
assert(vim.fn.maparg('<C-w>p', 'n') == '', 'Neovim Ctrl-w p must stay native')

enqueue('pane 12\n')
enqueue('')
nav.jump()
assert(table.concat(calls[3], ' ') == 'zellij pipe --name navi-zellij-pair -- v1 get 0')
assert(table.concat(calls[4], ' ') == 'zellij action focus-pane-id terminal_12')

enqueue('ok\n')
nav.unpair()
assert(table.concat(calls[5], ' ') == 'zellij pipe --name navi-zellij-pair -- v1 unpair 0')
assert(notices[#notices] and notices[#notices]:find('Unpaired', 1, true), 'unpairing should be confirmed')

enqueue('none\n')
nav.jump()
assert(#calls == 6, 'an expired pair must not focus a pane')

enqueue('')
nav.jump()
assert(#calls == 7, 'a missing plugin must not focus a pane')
assert(#notices > 0, 'failures must be shown to the user')

vim.ui.select = function(_, _, callback)
    callback(nil)
end
enqueue(vim.json.encode(panes))
nav.pair()
assert(#calls == 8, 'cancelling the picker must not send a pair')

nav.setup({
    in_zellij = function()
        return false
    end,
    exec = function()
        error('outside Zellij')
    end,
})
nav.pair()
nav.jump()
nav.unpair()
assert(#calls == 8, 'outside Zellij must not run CLI commands')

vim.ui.select, vim.notify = old_select, old_notify
vim.env.ZELLIJ_PANE_ID = old_pane
print('Neovim paired-pane tests passed')
