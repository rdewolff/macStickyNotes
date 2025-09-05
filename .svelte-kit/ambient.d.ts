
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * Environment variables [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env`. Like [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), this module cannot be imported into client-side code. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * _Unlike_ [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), the values exported from this module are statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * ```ts
 * import { API_KEY } from '$env/static/private';
 * ```
 * 
 * Note that all environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * 
 * ```
 * MY_FEATURE_FLAG=""
 * ```
 * 
 * You can override `.env` values from the command line like so:
 * 
 * ```sh
 * MY_FEATURE_FLAG="enabled" npm run dev
 * ```
 */
declare module '$env/static/private' {
	export const AS: string;
	export const HOST_PATH: string;
	export const NIX_HARDENING_ENABLE: string;
	export const enableParallelBuilding: string;
	export const AR: string;
	export const DIRENV_LOG_FORMAT: string;
	export const DEVELOPER_DIR: string;
	export const NIX_PROFILES: string;
	export const TERM_PROGRAM: string;
	export const NODE: string;
	export const INIT_CWD: string;
	export const NIX_CC: string;
	export const SHELL: string;
	export const TERM: string;
	export const __sandboxProfile: string;
	export const buildCommandPath: string;
	export const depsHostHost: string;
	export const propagatedBuildInputs: string;
	export const NM: string;
	export const HOMEBREW_REPOSITORY: string;
	export const TMPDIR: string;
	export const npm_config_global_prefix: string;
	export const NIX_ENFORCE_NO_NATIVE: string;
	export const TERM_PROGRAM_VERSION: string;
	export const NIX_IGNORE_LD_THROUGH_GCC: string;
	export const SIZE: string;
	export const MallocNanoZone: string;
	export const ORIGINAL_XDG_CURRENT_DESKTOP: string;
	export const SDKROOT: string;
	export const SOURCE_DATE_EPOCH: string;
	export const TAURI_CLI_VERBOSITY: string;
	export const COLOR: string;
	export const TAURI_ENV_DEBUG: string;
	export const builder: string;
	export const NIX_CFLAGS_COMPILE: string;
	export const cmakeFlags: string;
	export const depsTargetTarget: string;
	export const npm_config_noproxy: string;
	export const npm_config_local_prefix: string;
	export const USER: string;
	export const name: string;
	export const LS_COLORS: string;
	export const NIX_DONT_SET_RPATH: string;
	export const TAURI_ENV_TARGET_TRIPLE: string;
	export const TEMP: string;
	export const __impureHostDeps: string;
	export const depsBuildBuild: string;
	export const COMMAND_MODE: string;
	export const nativeBuildInputs: string;
	export const npm_config_globalconfig: string;
	export const SSH_AUTH_SOCK: string;
	export const NIX_STORE: string;
	export const __CF_USER_TEXT_ENCODING: string;
	export const mesonFlags: string;
	export const enableParallelChecking: string;
	export const npm_execpath: string;
	export const stdenv: string;
	export const PAGER: string;
	export const TAURI_ENV_PLATFORM: string;
	export const __darwinAllowLocalNetworking: string;
	export const STRINGS: string;
	export const LSCOLORS: string;
	export const WEZTERM_EXECUTABLE_DIR: string;
	export const ZERO_AR_DATE: string;
	export const system: string;
	export const doInstallCheck: string;
	export const NIX_LDFLAGS: string;
	export const PATH: string;
	export const __structuredAttrs: string;
	export const passAsFile: string;
	export const outputs: string;
	export const TEMPDIR: string;
	export const _: string;
	export const npm_package_json: string;
	export const LD: string;
	export const NIX_BUILD_TOP: string;
	export const TAURI_ENV_FAMILY: string;
	export const TAURI_ENV_PLATFORM_VERSION: string;
	export const TAURI_UPDATER_PLUGIN_CONFIG: string;
	export const __CFBundleIdentifier: string;
	export const npm_config_init_module: string;
	export const npm_config_userconfig: string;
	export const PWD: string;
	export const npm_command: string;
	export const STRIP: string;
	export const EDITOR: string;
	export const NIX_CC_WRAPPER_TARGET_HOST_arm64_apple_darwin: string;
	export const npm_lifecycle_event: string;
	export const LANG: string;
	export const npm_package_name: string;
	export const NIX_BINTOOLS_WRAPPER_TARGET_HOST_arm64_apple_darwin: string;
	export const WEZTERM_PANE: string;
	export const depsBuildTarget: string;
	export const depsHostHostPropagated: string;
	export const NODE_PATH: string;
	export const PATH_LOCALE: string;
	export const shell: string;
	export const VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
	export const XPC_FLAGS: string;
	export const npm_config_npm_version: string;
	export const NIX_SSL_CERT_FILE: string;
	export const TAURI_ENV_ARCH: string;
	export const doCheck: string;
	export const depsBuildTargetPropagated: string;
	export const npm_config_node_gyp: string;
	export const CXX: string;
	export const WEZTERM_UNIX_SOCKET: string;
	export const XPC_SERVICE_NAME: string;
	export const npm_package_version: string;
	export const OBJCOPY: string;
	export const out: string;
	export const CONFIG_SHELL: string;
	export const HOME: string;
	export const SHLVL: string;
	export const configureFlags: string;
	export const VSCODE_GIT_ASKPASS_MAIN: string;
	export const HOMEBREW_PREFIX: string;
	export const __propagatedImpureHostDeps: string;
	export const __propagatedSandboxProfile: string;
	export const IN_NIX_SHELL: string;
	export const patches: string;
	export const LESS: string;
	export const LOGNAME: string;
	export const MACOSX_DEPLOYMENT_TARGET: string;
	export const NIX_DONT_SET_RPATH_FOR_BUILD: string;
	export const TMP: string;
	export const npm_config_cache: string;
	export const NIX_APPLE_SDK_VERSION: string;
	export const NIX_NO_SELF_RPATH: string;
	export const npm_lifecycle_script: string;
	export const FZF_CTRL_T_COMMAND: string;
	export const VSCODE_GIT_IPC_HANDLE: string;
	export const XDG_DATA_DIRS: string;
	export const strictDeps: string;
	export const FZF_DEFAULT_COMMAND: string;
	export const buildInputs: string;
	export const RANLIB: string;
	export const npm_config_user_agent: string;
	export const GIT_ASKPASS: string;
	export const HOMEBREW_CELLAR: string;
	export const INFOPATH: string;
	export const NIX_BUILD_CORES: string;
	export const VSCODE_GIT_ASKPASS_NODE: string;
	export const depsBuildBuildPropagated: string;
	export const propagatedNativeBuildInputs: string;
	export const CC: string;
	export const WEZTERM_EXECUTABLE: string;
	export const LD_DYLD_PATH: string;
	export const NIX_BINTOOLS: string;
	export const OBJDUMP: string;
	export const __HM_SESS_VARS_SOURCED: string;
	export const depsTargetTargetPropagated: string;
	export const COLORTERM: string;
	export const __HM_ZSH_SESS_VARS_SOURCED: string;
	export const enableParallelInstalling: string;
	export const npm_config_prefix: string;
	export const npm_node_execpath: string;
	export const NODE_ENV: string;
}

/**
 * Similar to [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private), except that it only includes environment variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Values are replaced statically at build time.
 * 
 * ```ts
 * import { PUBLIC_BASE_URL } from '$env/static/public';
 * ```
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to runtime environment variables, as defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * This module cannot be imported into client-side code.
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * console.log(env.DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` always includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 */
declare module '$env/dynamic/private' {
	export const env: {
		AS: string;
		HOST_PATH: string;
		NIX_HARDENING_ENABLE: string;
		enableParallelBuilding: string;
		AR: string;
		DIRENV_LOG_FORMAT: string;
		DEVELOPER_DIR: string;
		NIX_PROFILES: string;
		TERM_PROGRAM: string;
		NODE: string;
		INIT_CWD: string;
		NIX_CC: string;
		SHELL: string;
		TERM: string;
		__sandboxProfile: string;
		buildCommandPath: string;
		depsHostHost: string;
		propagatedBuildInputs: string;
		NM: string;
		HOMEBREW_REPOSITORY: string;
		TMPDIR: string;
		npm_config_global_prefix: string;
		NIX_ENFORCE_NO_NATIVE: string;
		TERM_PROGRAM_VERSION: string;
		NIX_IGNORE_LD_THROUGH_GCC: string;
		SIZE: string;
		MallocNanoZone: string;
		ORIGINAL_XDG_CURRENT_DESKTOP: string;
		SDKROOT: string;
		SOURCE_DATE_EPOCH: string;
		TAURI_CLI_VERBOSITY: string;
		COLOR: string;
		TAURI_ENV_DEBUG: string;
		builder: string;
		NIX_CFLAGS_COMPILE: string;
		cmakeFlags: string;
		depsTargetTarget: string;
		npm_config_noproxy: string;
		npm_config_local_prefix: string;
		USER: string;
		name: string;
		LS_COLORS: string;
		NIX_DONT_SET_RPATH: string;
		TAURI_ENV_TARGET_TRIPLE: string;
		TEMP: string;
		__impureHostDeps: string;
		depsBuildBuild: string;
		COMMAND_MODE: string;
		nativeBuildInputs: string;
		npm_config_globalconfig: string;
		SSH_AUTH_SOCK: string;
		NIX_STORE: string;
		__CF_USER_TEXT_ENCODING: string;
		mesonFlags: string;
		enableParallelChecking: string;
		npm_execpath: string;
		stdenv: string;
		PAGER: string;
		TAURI_ENV_PLATFORM: string;
		__darwinAllowLocalNetworking: string;
		STRINGS: string;
		LSCOLORS: string;
		WEZTERM_EXECUTABLE_DIR: string;
		ZERO_AR_DATE: string;
		system: string;
		doInstallCheck: string;
		NIX_LDFLAGS: string;
		PATH: string;
		__structuredAttrs: string;
		passAsFile: string;
		outputs: string;
		TEMPDIR: string;
		_: string;
		npm_package_json: string;
		LD: string;
		NIX_BUILD_TOP: string;
		TAURI_ENV_FAMILY: string;
		TAURI_ENV_PLATFORM_VERSION: string;
		TAURI_UPDATER_PLUGIN_CONFIG: string;
		__CFBundleIdentifier: string;
		npm_config_init_module: string;
		npm_config_userconfig: string;
		PWD: string;
		npm_command: string;
		STRIP: string;
		EDITOR: string;
		NIX_CC_WRAPPER_TARGET_HOST_arm64_apple_darwin: string;
		npm_lifecycle_event: string;
		LANG: string;
		npm_package_name: string;
		NIX_BINTOOLS_WRAPPER_TARGET_HOST_arm64_apple_darwin: string;
		WEZTERM_PANE: string;
		depsBuildTarget: string;
		depsHostHostPropagated: string;
		NODE_PATH: string;
		PATH_LOCALE: string;
		shell: string;
		VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
		XPC_FLAGS: string;
		npm_config_npm_version: string;
		NIX_SSL_CERT_FILE: string;
		TAURI_ENV_ARCH: string;
		doCheck: string;
		depsBuildTargetPropagated: string;
		npm_config_node_gyp: string;
		CXX: string;
		WEZTERM_UNIX_SOCKET: string;
		XPC_SERVICE_NAME: string;
		npm_package_version: string;
		OBJCOPY: string;
		out: string;
		CONFIG_SHELL: string;
		HOME: string;
		SHLVL: string;
		configureFlags: string;
		VSCODE_GIT_ASKPASS_MAIN: string;
		HOMEBREW_PREFIX: string;
		__propagatedImpureHostDeps: string;
		__propagatedSandboxProfile: string;
		IN_NIX_SHELL: string;
		patches: string;
		LESS: string;
		LOGNAME: string;
		MACOSX_DEPLOYMENT_TARGET: string;
		NIX_DONT_SET_RPATH_FOR_BUILD: string;
		TMP: string;
		npm_config_cache: string;
		NIX_APPLE_SDK_VERSION: string;
		NIX_NO_SELF_RPATH: string;
		npm_lifecycle_script: string;
		FZF_CTRL_T_COMMAND: string;
		VSCODE_GIT_IPC_HANDLE: string;
		XDG_DATA_DIRS: string;
		strictDeps: string;
		FZF_DEFAULT_COMMAND: string;
		buildInputs: string;
		RANLIB: string;
		npm_config_user_agent: string;
		GIT_ASKPASS: string;
		HOMEBREW_CELLAR: string;
		INFOPATH: string;
		NIX_BUILD_CORES: string;
		VSCODE_GIT_ASKPASS_NODE: string;
		depsBuildBuildPropagated: string;
		propagatedNativeBuildInputs: string;
		CC: string;
		WEZTERM_EXECUTABLE: string;
		LD_DYLD_PATH: string;
		NIX_BINTOOLS: string;
		OBJDUMP: string;
		__HM_SESS_VARS_SOURCED: string;
		depsTargetTargetPropagated: string;
		COLORTERM: string;
		__HM_ZSH_SESS_VARS_SOURCED: string;
		enableParallelInstalling: string;
		npm_config_prefix: string;
		npm_node_execpath: string;
		NODE_ENV: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * Similar to [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), but only includes variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Note that public dynamic environment variables must all be sent from the server to the client, causing larger network requests — when possible, use `$env/static/public` instead.
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.PUBLIC_DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
