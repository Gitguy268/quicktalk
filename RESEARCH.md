# Static reference research

REA 6.7.0 was used for local static analysis. No proprietary code was copied into QuickTalk, and neither application was launched for this analysis. Findings are architecture evidence, not recovery of remote services.
## Wispr

Evidence ID: `ev_938e872facb0881b6b9a117742bee0c728a4d1d575f6c215cf6b4a549e5aaa7c`

Archive SHA-256: `dc56b610fb46d34dabf7871a270f307c665371de3f28b492c687eca2af09faf4`

Summary:

```json
{
  "browser_windows": 0,
  "explicit_web_preferences": 0,
  "preload_entrypoints": 0,
  "context_bridge_apis": 36,
  "exposed_api_members": 0,
  "ipc": {
    "operations": 207,
    "literal_channels": 10,
    "dynamic_channel_operations": 90,
    "renderer_transmissions": 180,
    "renderer_listeners": 27,
    "main_handlers": 0,
    "paired_renderer_transmissions": 0,
    "ambiguous_renderer_transmissions": 0,
    "unpaired_literal_renderer_transmissions": 108
  },
  "sender_validation_observations": 1,
  "utility_processes": 0,
  "resolved_utility_entrypoints": 0,
  "native_addon_bindings": 0,
  "resolved_native_addon_bindings": 0
}
```

Statistics:

```json
{
  "relevant_files": 183,
  "nested_asar_containers": 0,
  "text_bytes_read": 48511973,
  "invalid_utf8_files": 0,
  "parsed_javascript_files": 165,
  "skipped_javascript_files": 2,
  "visited_ast_nodes": 3313992,
  "findings": 7810,
  "modules": 2293,
  "parse_failures": 0
}
```

Limitations:

- .webpack/main/903.js: Resource constraint during analyze_javascript_application: Owned JavaScript worker exhausted its heap while analyzing .webpack/main/903.js during semantic
- .webpack/main/incrementalOpusEncoder.worker.js: Resource constraint during analyze_javascript_application: Owned JavaScript worker exhausted its heap while analyzing .webpack/main/incrementalOpusEncoder.worker.js during semantic
- .webpack/main/index.js: Resource constraint during analyze_javascript_application: Owned JavaScript worker exhausted its heap while analyzing .webpack/main/index.js during static
- .webpack/main/opusEncoder.worker.js: Resource constraint during analyze_javascript_application: Owned JavaScript worker exhausted its heap while analyzing .webpack/main/opusEncoder.worker.js during semantic
- .webpack/renderer/feature_tour/preload.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/feature_tour/preload.js was not started
- .webpack/renderer/hub/index.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/hub/index.js was not started
- .webpack/renderer/hub/preload.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/hub/preload.js was not started
- .webpack/renderer/meeting_ax_inspector/index.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/meeting_ax_inspector/index.js was not started
- .webpack/renderer/meeting_ax_inspector/preload.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/meeting_ax_inspector/preload.js was not started
- .webpack/renderer/meeting_recorder/index.js: Resource constraint during analyze_javascript_application: The parent reserved its remaining result capacity for static observations; semantic analysis of .webpack/renderer/meeting_recorder/index.js was not started

Integrity contradictions were retained as untrusted observations where reported. Static graph relationships do not prove runtime behavior.

## Granola

Evidence ID: `ev_f57331bb5564062b7f9f78fd83c71b77e825dbd8ef778dea04ec8d6637876246`

Archive SHA-256: `45364d23e09cac24136c531c4dfaf41026ef13a0d3e4e291731a99f547585a4f`

Summary:

```json
{
  "browser_windows": 0,
  "explicit_web_preferences": 0,
  "preload_entrypoints": 0,
  "context_bridge_apis": 2,
  "exposed_api_members": 365,
  "ipc": {
    "operations": 12,
    "literal_channels": 3,
    "dynamic_channel_operations": 9,
    "renderer_transmissions": 8,
    "renderer_listeners": 4,
    "main_handlers": 0,
    "paired_renderer_transmissions": 0,
    "ambiguous_renderer_transmissions": 0,
    "unpaired_literal_renderer_transmissions": 0
  },
  "sender_validation_observations": 0,
  "utility_processes": 0,
  "resolved_utility_entrypoints": 0,
  "native_addon_bindings": 0,
  "resolved_native_addon_bindings": 0
}
```

Statistics:

```json
{
  "relevant_files": 900,
  "nested_asar_containers": 0,
  "text_bytes_read": 29528975,
  "invalid_utf8_files": 0,
  "parsed_javascript_files": 785,
  "skipped_javascript_files": 1,
  "visited_ast_nodes": 6853740,
  "findings": 12940,
  "modules": 6,
  "parse_failures": 0
}
```

Limitations:

- Artifact integrity metadata contradicts observed bytes at node_modules/@napi-rs/xattr-darwin-arm64/xattr.darwin-arm64.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/@napi-rs/xattr-darwin-x64/xattr.darwin-x64.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/better-sqlite3-multiple-ciphers/build/Release/better_sqlite3.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/better-sqlite3-multiple-ciphers/build/Release/test_extension.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/electron-click-drag-plugin/build/Release/darwin-arm64/drag.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/electron-click-drag-plugin/build/Release/darwin-x64/drag.node; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/keyspy/runtime/MacKeyServer; the observed bytes are untrusted.
- Artifact integrity metadata contradicts observed bytes at node_modules/registry-js/build/Release/registry.node; the observed bytes are untrusted.
- CommonJS and ESM binding relationships were recovered from inert syntax and resolved only within the inventoried artifact container.
- Dynamic IPC channel expressions remain unresolved and are never paired by textual similarity.

Integrity contradictions were retained as untrusted observations where reported. Static graph relationships do not prove runtime behavior.

## QuickTalk implementation decisions

Use separate capture/transcription work from the UI; preserve audio on failed processing; keep source tracks distinct; provide a minimal foreground view and background tray access. These are independent design choices informed by the product requirements and public/native APIs. REA output is incomplete where parsing or memory budgets were exhausted.
