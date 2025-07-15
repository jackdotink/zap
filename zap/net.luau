local hello_world = zap.event("client", { zap.string.utf8() })

return {
	hello_world = hello_world,
}