local hello_world = zap.reliable("client", { zap.string.utf8() })

return {
	hello_world = hello_world,
}