local remote = zap.remote()

local hello_world = remote:event("client", { zap.string.utf8() })

return {
	hello_world = hello_world,
}