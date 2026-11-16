# The Model Context Protocol: the facts that OpenCraft uses

Last update: 2026-11-16

The server `opencraft mcp` follows the Model Context Protocol over stdio,
revision `2025-11-25` and revision `2026-07-28`
([decision 0040](../decisions/0040-control-socket.md), section 6). This entry
holds the protocol facts, their sources, and the two choices of OpenCraft.

## The two revisions (2026-11-16)

| Point | `2025-11-25` | `2026-07-28` | Confidence | Source |
|---|---|---|---|---|
| Opening | `initialize`, then `notifications/initialized` | No handshake. Each request has its version | Source | [Lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), [changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) |
| Version of a request | In `initialize`; the answer names the version of the server | `_meta.io.modelcontextprotocol/protocolVersion` in each request | Source | [Versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning) |
| Capabilities of a client | In `initialize` | `_meta.io.modelcontextprotocol/clientCapabilities` in each request | Source | [Base protocol: `_meta`](https://modelcontextprotocol.io/specification/2026-07-28/basic#meta) |
| A missing required `_meta` field | Not applicable | `-32602` (Invalid params) | Source | same page |
| The type of a result | None | `resultType`, `"complete"` for a finished request | Source | [Result type](https://modelcontextprotocol.io/specification/2026-07-28/basic#resulttype) |
| Caching of a list | None | `ttlMs` and `cacheScope` on `tools/list` and `server/discover` | Source | [Caching](https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching) |
| Discovery | None | `server/discover` is required. The result has `supportedVersions`, `capabilities`, `instructions`, `ttlMs`, `cacheScope`, and `_meta.io.modelcontextprotocol/serverInfo` | Source | [Discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/discover) |
| `ping` | A method. A client may send it before the answer to `initialize` | Removed | Source | [Lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), [changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) |

The two revisions do not agree on the bytes of a request or of a result, thus
a server selects one format from how the client opens. A client that has both
revisions sends `server/discover` first: a `DiscoverResult` or
`UnsupportedProtocolVersionError` means a modern server, and any other answer
means a legacy server that needs `initialize`. A client must not select the
legacy handshake from one error code alone
([stdio: backward compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio#backward-compatibility)).
Confidence: Source.

## The transport and the messages (2026-11-16)

| Fact | Confidence | Source |
|---|---|---|
| A message is one JSON-RPC message on one line, on the standard input and the standard output. The standard output carries messages only; a message for a person goes to the standard error | Source | [stdio](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio), [stdio 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) |
| A request has `jsonrpc: "2.0"`, an `id` that is a text or a whole number, a `method`, and optional `params` that are an object | Source | [Base protocol: requests](https://modelcontextprotocol.io/specification/2026-07-28/basic#requests) |
| A response gets no answer. A notification gets no answer | Source | JSON-RPC 2.0, [Base protocol: requests](https://modelcontextprotocol.io/specification/2026-07-28/basic#requests) |
| A line that is not JSON is `-32700` (Parse error). A message that is not a correct request is `-32600` (Invalid request) | Source | JSON-RPC 2.0 |
| After `notifications/cancelled`, a server should stop the work of the request as soon as practical. On stdio there is no stream for one request to close | Source | [stdio: cancellation](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio#cancellation) |

## The tools and their errors (2026-11-16)

| Fact | Confidence | Source |
|---|---|---|
| A server must validate all tool inputs | Source | [Tools: security](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#security-considerations) |
| A protocol error (`-32602` for an unknown tool or a request that does not agree with `CallToolRequest`) is for the structure of the request. It is not for a wrong argument of a valid tool | Source | [Tools: error handling](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#error-handling) |
| A wrong argument, a failure of an API, and a business error are a tool result with `isError: true`. A client should give that result to the model; it may give a protocol error | Source | same page |
| `arguments` of `tools/call` is an object. `null` is not an object | Source | [Schema: `CallToolRequest`](https://modelcontextprotocol.io/specification/2026-07-28/schema#calltoolrequest) |
| `destructiveHint: false` means "the tool performs only additive updates". It does not mean "the user can undo it". The default is `true` | Source | [Schema: `ToolAnnotations`](https://modelcontextprotocol.io/specification/2026-07-28/schema#toolannotations) |
| `idempotentHint` and `destructiveHint` are meaningful only when `readOnlyHint` is `false` | Source | same page |

## Version negotiation (2026-11-16)

| Fact | Confidence | Source |
|---|---|---|
| A version that the server does not have is `-32022` (`UnsupportedProtocolVersionError`). `data.supported` lists the versions that the server does support, and `data.requested` names the version of the request | Source | [Version negotiation](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning#protocol-version-negotiation) |
| The example of the specification lists a legacy version in `data.supported`: `["2026-07-28", "2025-11-25"]`. A client selects a version from that list and tries again. It must not fall back to `initialize` after this error | Source | same page, [stdio: backward compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio#backward-compatibility) |
| The example of the specification for `server/discover` names one version in `supportedVersions`. The official TypeScript SDK also lists only modern revisions there | Source | [Discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/discover), the SDK `packages/server/src/server/server.ts` |

**The choices of OpenCraft.** `data.supported` names both versions, as the
example of the specification does. `supportedVersions` of `server/discover`
names only `2026-07-28`, because a request of `2025-11-25` has a different
format and a client that selects it does not send a modern request.
Confidence: Inference from the pages above.

## The limits of Claude Code (2026-11-16)

| Fact | Confidence | Source |
|---|---|---|
| Claude Code cuts server instructions, and the description of a tool that loads at the start, after 4,096 characters. The model sees only the text before the cut | Source | [Claude Code: MCP](https://code.claude.com/docs/en/mcp#tool-search-for-mcp-server-authors) |
| The description of a tool that Claude Code loads through tool search has a limit of 16,384 characters | Source | same page |
| Before Claude Code v2.1.296, the shorter limit was 2,048 characters | Source | same page |
| The specification says that `instructions` "should not duplicate information already in tool descriptions" | Source | [Schema: `DiscoverResult`](https://modelcontextprotocol.io/specification/2026-07-28/schema#discoverresult) |
| Claude Code 2.1.325 has `2025-11-25`, `2026-07-28`, and `server/discover` in its program | Measured | `strings` on the installed `claude` program |
| codex-cli 0.169.0 has `2025-11-25`, `2026-07-28`, and `server/discover` in its program | Measured | `strings` on the installed codex program |

## Not known

- Which revision and which version a real session of Claude Code or of the
  Codex CLI sends to this server. No session was recorded.
- Whether a real client gives a tool result with `isError: true` to the model
  in each revision. The specification tells a client to do it; no session was
  observed.
- Whether a client sends `arguments: null` or another form that the schema
  does not permit.

## Sources

- Model Context Protocol, specification `2025-11-25` and `2026-07-28`:
  <https://modelcontextprotocol.io/specification/2025-11-25> and
  <https://modelcontextprotocol.io/specification/2026-07-28>.
- Claude Code, "Connect Claude Code to tools via MCP":
  <https://code.claude.com/docs/en/mcp>.
- The programs of Claude Code 2.1.325 and of codex-cli 0.169.0 on this
  computer.
