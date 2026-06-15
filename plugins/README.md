# Runtime Plugin Workspace

Purpose: application/runtime plugin source packages and installable extension packages governed by the SDKWork project-root dictionary.

Owner: SDKWork kernel maintainers.

Allowed content: runtime plugin source roots, plugin specs, plugin manifests, conformance fixtures, and plugin package documentation.

Forbidden content: repository agent plugins, `.codex-plugin` bundles for local agent tooling, generated SDK output, third-party reference trees, live secrets, and runtime databases.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../specs/KERNEL_PLUGIN_SPEC.md`, `../sdkwork-specs/CODE_STYLE_SPEC.md`, and `../sdkwork-specs/TEST_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs` and `node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs`.
