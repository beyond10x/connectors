# `connectors-mcp-launch` CLI contract

Local MCP stdio binding over admitted Connectors operations

This generated package installs an unavailable handler. Application behavior enters through the `Handler` seam; schema-selected calls also require `DynamicValidator`. See `help.txt` for process options and `binding.json` for resolved types and targets.

## Commands

### `server`

Serve MCP on stdin and stdout as the admitted local owner

Callable: `local-server`. Input: `connectors_mcp.launch.LocalLaunchInput`. Result: `connectors_mcp.launch.LocalLaunchCompletion`.

Error `failure`: `connectors_mcp.launch.LocalLaunchFailure`.

## Runtime obligations

- handler:local-server: implement the owner-qualified callable and its declared result/error contract
