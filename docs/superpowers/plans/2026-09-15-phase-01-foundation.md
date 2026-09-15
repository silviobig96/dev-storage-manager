# Phase 01 Application Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a testable macOS-first Tauri 2 application shell with a React/TypeScript frontend, layered Rust foundation, local SQLite migration boundary, and continuous integration.

**Architecture:** A thin Tauri crate owns process startup and typed IPC. An application crate coordinates platform and persistence ports, a core crate owns shared serialized domain values, a macOS platform crate implements the initial adapter, and a persistence crate owns SQLite. The frontend calls one read-only health query to prove the complete boundary without introducing product features from later phases.

**Tech Stack:** Tauri 2, React, TypeScript, Vite, Rust 2021, SQLite through `rusqlite`, Vitest, Testing Library, ESLint, Prettier, Cargo test/clippy/fmt, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-15-dev-storage-manager-design.md`

## Global Constraints

- Phase 01 is application foundation only.
- Do not implement providers, workspace behavior, scanning, cleanup planning, cleanup execution, monitoring, notifications, privileged helpers, Full Disk Access behavior, or Windows platform code.
- No automatic deletion or scheduled cleanup is permitted.
- Do not hardcode `~/Repositories` or another workspace root.
- Safety policy remains a Rust-core responsibility when introduced in its roadmap phase.
- Keep Tauri commands thin; frontend code must not access SQLite directly.
- Keep macOS-specific behavior behind the platform adapter.
- Use npm and commit `package-lock.json`; use Cargo and commit `Cargo.lock`.
- Resolve frontend packages with `--save-exact` during this implementation so `package.json` and the lockfile capture the selected versions.
- Use Rust 2021 and the stable Rust channel; the lockfile captures compatible crate versions.
- Run focused tests before workspace-wide checks.
- The only IPC use case in this phase is a read-only application-health query.

---

## Intended file map

The implementation creates or modifies only these paths in addition to the Phase 00 documentation:

```text
.editorconfig
.gitignore
.nvmrc
.prettierignore
.prettierrc.json
Cargo.lock
Cargo.toml
eslint.config.js
index.html
package-lock.json
package.json
rust-toolchain.toml
tsconfig.app.json
tsconfig.json
tsconfig.node.json
vite.config.ts
.github/workflows/ci.yml
assets/app-icon.svg
crates/core/Cargo.toml
crates/core/src/lib.rs
crates/platform/Cargo.toml
crates/platform/src/lib.rs
crates/platform/src/macos.rs
crates/persistence/Cargo.toml
crates/persistence/migrations/0001_application_metadata.sql
crates/persistence/src/lib.rs
crates/application/Cargo.toml
crates/application/src/lib.rs
src/main.tsx
src/App.tsx
src/App.css
src/index.css
src/vite-env.d.ts
src/lib/desktop.ts
src/lib/desktop.test.ts
src/test/setup.ts
src/App.test.tsx
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/tauri.conf.json
src-tauri/capabilities/default.json
src-tauri/icons/                 generated Tauri icon outputs
src-tauri/src/commands.rs
src-tauri/src/lib.rs
src-tauri/src/main.rs
```

No `windows` module, scanner module, provider module, cleanup module, monitor module, network client, telemetry client, or filesystem permission entitlement is created.

---

### Task 1: Reproducible frontend and Rust workspace baseline

**Files:**

- Create: `.editorconfig`
- Create: `.gitignore`
- Create: `.nvmrc`
- Create: `.prettierignore`
- Create: `.prettierrc.json`
- Create: `package.json`
- Create: `package-lock.json` through npm
- Create: `eslint.config.js`
- Create: `tsconfig.json`
- Create: `tsconfig.app.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `rust-toolchain.toml`
- Create: `Cargo.toml`

**Interfaces:**

- Consumes: Node.js 22 LTS, npm, stable Rust, Xcode command-line tools.
- Produces: npm scripts `dev`, `build`, `test`, `test:run`, `lint`, `format`, `format:check`, and `tauri`; a Cargo workspace containing `crates/core`, `crates/platform`, `crates/persistence`, `crates/application`, and `src-tauri`.

- [ ] **Step 1: Verify the clean Phase 00 baseline and required tools**

Run:

```bash
git status --short
node --version
npm --version
rustc --version
cargo --version
xcode-select -p
```

Expected: only the intended Phase 00 commit is present, Node reports major version 22, Rust/Cargo report the stable toolchain, and `xcode-select` prints an installed developer path. Stop on unrelated changes rather than overwriting them.

- [ ] **Step 2: Create the root configuration files**

Use `.nvmrc`:

```text
22
```

Use `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

Use root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = []

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
```

Use `.gitignore`:

```gitignore
node_modules/
dist/
target/
.DS_Store
*.db
*.db-shm
*.db-wal
```

Use `.editorconfig`:

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 2
trim_trailing_whitespace = true

[*.rs]
indent_size = 4

[*.md]
trim_trailing_whitespace = false
```

Use `.prettierrc.json`:

```json
{
  "semi": true,
  "singleQuote": true,
  "trailingComma": "all"
}
```

Use `.prettierignore`:

```text
dist
node_modules
src-tauri/icons
target
Cargo.lock
package-lock.json
```

- [ ] **Step 3: Initialize and pin frontend tooling**

Run:

```bash
npm init -y
npm install --save-exact react@latest react-dom@latest @tauri-apps/api@latest
npm install --save-dev --save-exact @eslint/js@latest @tauri-apps/cli@latest @testing-library/jest-dom@latest @testing-library/react@latest @testing-library/user-event@latest @types/node@latest @types/react@latest @types/react-dom@latest @vitejs/plugin-react@latest eslint@latest eslint-plugin-react-hooks@latest eslint-plugin-react-refresh@latest globals@latest jsdom@latest prettier@latest typescript@latest typescript-eslint@latest vite@latest vitest@latest
npm pkg set type=module
npm pkg set scripts.dev=vite
npm pkg set scripts.build="tsc -b && vite build"
npm pkg set scripts.test=vitest
npm pkg set scripts.test:run="vitest run"
npm pkg set scripts.lint="eslint . --max-warnings 0"
npm pkg set scripts.format="prettier --write ."
npm pkg set scripts.format:check="prettier --check ."
npm pkg set scripts.tauri=tauri
```

Expected: npm writes exact versions to `package.json` and a deterministic `package-lock.json`.

- [ ] **Step 4: Add TypeScript, Vite, and ESLint configuration**

Configure `tsconfig.json` with project references to `tsconfig.app.json` and `tsconfig.node.json`. Configure `tsconfig.app.json` for strict DOM/ES2022 React JSX compilation with `noEmit`, and `tsconfig.node.json` for `vite.config.ts` and `eslint.config.js`.

Use `tsconfig.json`:

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.node.json" }
  ]
}
```

Use `tsconfig.app.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "Bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUncheckedIndexedAccess": true
  },
  "include": ["src"]
}
```

Use `tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "target": "ES2023",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "Bundler",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "types": ["node"]
  },
  "include": ["vite.config.ts", "eslint.config.js"]
}
```

Use `vite.config.ts`:

```ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    restoreMocks: true,
  },
});
```

Use `eslint.config.js`:

```js
import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'src-tauri/icons', 'target'] },
  {
    files: ['src/**/*.{ts,tsx}'],
    extends: [js.configs.recommended, ...tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
    },
  },
);
```

- [ ] **Step 5: Verify tool configuration parses**

Run:

```bash
npm exec vite -- --version
npm exec eslint -- --version
npm exec prettier -- --version
cargo metadata --no-deps --format-version 1
```

Expected: all four commands exit successfully. Cargo reports an empty but valid workspace until the internal crates are added in later tasks.

- [ ] **Step 6: Commit the reproducible baseline**

```bash
git add .editorconfig .gitignore .nvmrc .prettierignore .prettierrc.json Cargo.toml eslint.config.js package.json package-lock.json rust-toolchain.toml tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts
git commit -m "build: establish workspace toolchains"
```

---

### Task 2: Shared Rust core and macOS platform boundary

**Files:**

- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/platform/Cargo.toml`
- Create: `crates/platform/src/lib.rs`
- Create: `crates/platform/src/macos.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: workspace `serde` and `thiserror` dependencies.
- Produces: `dev_storage_core::{AppHealth, Platform}` and `dev_storage_platform::{PlatformAdapter, PlatformError, MacOsAdapter}`.

- [ ] **Step 1: Create crate manifests and write failing core serialization tests**

`crates/core/Cargo.toml` defines package `dev-storage-core` and depends on workspace `serde`; its dev-dependencies include `serde_json = "1"`.

Add `crates/core` and `crates/platform` to the root workspace `members` array in dependency order.

Start `crates/core/src/lib.rs` with tests that require the absent types:

```rust
#[cfg(test)]
mod tests {
    use super::{AppHealth, Platform};

    #[test]
    fn health_contract_serializes_with_stable_field_names() {
        let health = AppHealth {
            app_name: "dev-storage-manager".into(),
            app_version: "0.1.0".into(),
            platform: Platform::MacOs,
            database_schema_version: 1,
        };

        assert_eq!(
            serde_json::to_value(health).unwrap(),
            serde_json::json!({
                "appName": "dev-storage-manager",
                "appVersion": "0.1.0",
                "platform": "macos",
                "databaseSchemaVersion": 1
            })
        );
    }
}
```

- [ ] **Step 2: Run the focused test and confirm the expected failure**

Run:

```bash
cargo test -p dev-storage-core health_contract_serializes_with_stable_field_names
```

Expected: compilation fails because `AppHealth` and `Platform` are not defined.

- [ ] **Step 3: Implement the minimal shared values**

Add `Platform` with only `MacOs`, serialized as `macos`, and `AppHealth` with the four tested fields. Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, and `Deserialize`; use `#[serde(rename_all = "camelCase")]` for the health struct. Do not add Windows, scan, provider, workspace, cleanup, or monitor types.

- [ ] **Step 4: Run the focused core test**

Run:

```bash
cargo test -p dev-storage-core health_contract_serializes_with_stable_field_names
```

Expected: one focused test passes.

- [ ] **Step 5: Write a failing platform adapter test**

`crates/platform/Cargo.toml` defines package `dev-storage-platform` and depends on `dev-storage-core` and workspace `thiserror`.

In `crates/platform/src/lib.rs`, declare `mod macos;`, re-export `MacOsAdapter`, and write:

```rust
#[cfg(test)]
mod tests {
    use super::{MacOsAdapter, PlatformAdapter};
    use dev_storage_core::Platform;

    #[test]
    fn macos_adapter_reports_only_its_platform_identity() {
        assert_eq!(MacOsAdapter.platform(), Platform::MacOs);
    }
}
```

- [ ] **Step 6: Run the platform test and confirm the expected failure**

Run:

```bash
cargo test -p dev-storage-platform macos_adapter_reports_only_its_platform_identity
```

Expected: compilation fails because the adapter trait and implementation are absent.

- [ ] **Step 7: Implement the minimal adapter boundary**

Define:

```rust
pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> Platform;
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("platform capability is unavailable: {0}")]
    CapabilityUnavailable(&'static str),
}
```

In `macos.rs`, define the unit struct `MacOsAdapter` and implement `platform()` as `Platform::MacOs`. Do not add filesystem access or conditional Windows modules.

- [ ] **Step 8: Verify and commit the core/platform boundary**

Run:

```bash
cargo fmt --all --check
cargo test -p dev-storage-core
cargo test -p dev-storage-platform
cargo clippy -p dev-storage-core -p dev-storage-platform --all-targets -- -D warnings
```

Expected: formatting, tests, and clippy pass.

```bash
git add Cargo.toml Cargo.lock crates/core crates/platform
git commit -m "feat: define core and macOS platform boundaries"
```

---

### Task 3: SQLite migration and persistence boundary

**Files:**

- Create: `crates/persistence/Cargo.toml`
- Create: `crates/persistence/migrations/0001_application_metadata.sql`
- Create: `crates/persistence/src/lib.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: SQLite through `rusqlite` and ordered migrations through `rusqlite_migration`.
- Produces: `dev_storage_persistence::{Database, PersistenceError}`, with `Database::open`, `Database::open_in_memory`, and `Database::schema_version`.

- [ ] **Step 1: Write failing migration tests**

Define the crate manifest with `rusqlite` using the `bundled` feature, `rusqlite_migration`, and workspace `thiserror`. Begin `src/lib.rs` with:

Add `crates/persistence` to the root workspace `members` array.

```rust
#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn fresh_database_applies_the_first_schema_version() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(database.schema_version().unwrap(), 1);
    }

    #[test]
    fn migration_is_repeatable_for_an_existing_connection() {
        let mut database = Database::open_in_memory().unwrap();
        database.apply_migrations().unwrap();
        assert_eq!(database.schema_version().unwrap(), 1);
    }
}
```

- [ ] **Step 2: Run the focused tests and confirm the expected failure**

Run:

```bash
cargo test -p dev-storage-persistence fresh_database_applies_the_first_schema_version
```

Expected: compilation fails because `Database` is absent.

- [ ] **Step 3: Add the minimal application-owned schema**

Use `migrations/0001_application_metadata.sql`:

```sql
CREATE TABLE application_metadata (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
) STRICT;
```

Implement `Database` as the sole owner of a private `rusqlite::Connection`. `open(path: &Path)` and `open_in_memory()` both construct a mutable database, run one `M::up(include_str!(...))` migration transaction, and return it. `schema_version()` queries `PRAGMA user_version` or the migration crate's version table consistently with the migration mechanism and returns `u32`. `PersistenceError` wraps connection, query, and migration failures without exposing SQL or local file contents.

- [ ] **Step 4: Run focused persistence tests**

Run:

```bash
cargo test -p dev-storage-persistence
```

Expected: both tests pass against in-memory SQLite and create no repository database file.

- [ ] **Step 5: Add a temporary-file reopen test**

Add `tempfile` as a dev-dependency and test that opening the same `app.db` path twice leaves schema version `1` and retains an inserted `application_metadata` row. Add a narrow `set_metadata(&mut self, key: &str, value: &str)` and `metadata(&self, key: &str) -> Result<Option<String>, PersistenceError>` only to prove persistence and parameterized SQL.

Run:

```bash
cargo test -p dev-storage-persistence migration_persists_across_reopen
```

Expected: the reopen test passes and the temporary directory is removed by the test harness.

- [ ] **Step 6: Verify and commit persistence**

Run:

```bash
cargo fmt --all --check
cargo test -p dev-storage-persistence
cargo clippy -p dev-storage-persistence --all-targets -- -D warnings
```

Expected: all checks pass.

```bash
git add Cargo.toml Cargo.lock crates/persistence
git commit -m "feat: add SQLite migration foundation"
```

---

### Task 4: Application service crossing platform and persistence ports

**Files:**

- Create: `crates/application/Cargo.toml`
- Create: `crates/application/src/lib.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: `PlatformAdapter`, `Database`, and `AppHealth`.
- Produces: `ApplicationService<A>::new(adapter, database)` and `ApplicationService<A>::health() -> Result<AppHealth, ApplicationError>`.

- [ ] **Step 1: Write a failing application-service test**

Define the manifest for package `dev-storage-application` with path dependencies on core, persistence, and platform plus workspace `thiserror`.

Add `crates/application` to the root workspace `members` array.

Write:

```rust
#[cfg(test)]
mod tests {
    use super::ApplicationService;
    use dev_storage_core::Platform;
    use dev_storage_persistence::Database;
    use dev_storage_platform::PlatformAdapter;

    struct TestPlatform;

    impl PlatformAdapter for TestPlatform {
        fn platform(&self) -> Platform {
            Platform::MacOs
        }
    }

    #[test]
    fn health_combines_build_platform_and_schema_information() {
        let database = Database::open_in_memory().unwrap();
        let service = ApplicationService::new(TestPlatform, database);
        let health = service.health().unwrap();

        assert_eq!(health.app_name, "dev-storage-manager");
        assert_eq!(health.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(health.platform, Platform::MacOs);
        assert_eq!(health.database_schema_version, 1);
    }
}
```

- [ ] **Step 2: Run the focused test and confirm the expected failure**

Run:

```bash
cargo test -p dev-storage-application health_combines_build_platform_and_schema_information
```

Expected: compilation fails because `ApplicationService` is not defined.

- [ ] **Step 3: Implement the minimal application service**

Store the adapter and database as private fields. `health()` requests only the platform identity and database schema version, then constructs `AppHealth`. Define `ApplicationError` with transparent conversions from `PlatformError` where applicable and `PersistenceError`. Do not expose the connection, accept paths, or add other use cases.

- [ ] **Step 4: Verify the application layer and workspace**

Run:

```bash
cargo test -p dev-storage-application
cargo test --workspace --exclude dev-storage-manager
cargo clippy --workspace --all-targets --exclude dev-storage-manager -- -D warnings
```

Expected: all library tests and lints pass.

- [ ] **Step 5: Commit the application layer**

```bash
git add Cargo.toml Cargo.lock crates/application
git commit -m "feat: add application health service"
```

---

### Task 5: Thin Tauri shell and read-only command

**Files:**

- Create: `assets/app-icon.svg`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/icons/` through the Tauri icon generator
- Create: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/main.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: `ApplicationService<MacOsAdapter>` and Tauri's application-data directory resolver.
- Produces: IPC command `get_app_health` returning the serialized `AppHealth`; no other command is registered.

- [ ] **Step 1: Add the Tauri crate and minimal configuration**

Define package `dev-storage-manager` in `src-tauri/Cargo.toml`, use `tauri-build = { version = "2", features = [] }`, `tauri = { version = "2", features = [] }`, workspace `serde`, and path dependencies on all four internal crates. Set library crate types to `staticlib`, `cdylib`, and `rlib`.

Add `src-tauri` as the final root workspace member.

Use `build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

Configure `tauri.conf.json` with product name `dev-storage-manager`, version `0.1.0`, identifier `dev.devstoragemanager.app`, `beforeDevCommand` as `npm run dev`, `devUrl` as `http://localhost:1420`, `beforeBuildCommand` as `npm run build`, and `frontendDist` as `../dist`. Define one 1100 by 720 window with minimum size 760 by 520. Do not request filesystem, shell, updater, notification, or network plugins.

Use `capabilities/default.json` with the one main window and only Tauri core default permission. An empty product feature must not gain broad filesystem or shell permissions.

- [ ] **Step 2: Add and generate neutral foundation icons**

Create `assets/app-icon.svg` as an original simple disk-outline mark using only vector paths, a solid background, no text, and no third-party marks. Generate the standard Tauri icon set:

```bash
npm run tauri icon assets/app-icon.svg
```

Expected: Tauri creates its standard outputs under `src-tauri/icons/`, including macOS `icon.icns`. Review generated paths with `git status --short`; do not add product screenshots.

- [ ] **Step 3: Write a failing command-boundary unit test**

In `commands.rs`, define a private pure adapter function used by the Tauri command and test it:

```rust
#[cfg(test)]
mod tests {
    use super::health_payload;
    use dev_storage_application::ApplicationService;
    use dev_storage_core::Platform;
    use dev_storage_persistence::Database;
    use dev_storage_platform::PlatformAdapter;

    struct TestPlatform;

    impl PlatformAdapter for TestPlatform {
        fn platform(&self) -> Platform {
            Platform::MacOs
        }
    }

    #[test]
    fn command_adapter_returns_the_application_contract() {
        let service = ApplicationService::new(
            TestPlatform,
            Database::open_in_memory().unwrap(),
        );
        let payload = health_payload(&service).unwrap();
        assert_eq!(payload.database_schema_version, 1);
    }
}
```

- [ ] **Step 4: Run the focused test and confirm the expected failure**

Run:

```bash
cargo test -p dev-storage-manager command_adapter_returns_the_application_contract
```

Expected: compilation fails because `health_payload` and shell setup are incomplete.

- [ ] **Step 5: Implement shell startup and the command**

Define a serializable `CommandError { code: &'static str, message: String }` and map application errors to code `application_unavailable`. `health_payload` calls only `service.health()`.

The `#[tauri::command] get_app_health` function accepts `State<'_, Mutex<ApplicationService<MacOsAdapter>>>`, locks it, and returns the same typed payload. `lib.rs::run()` creates the app-data directory, opens `dev-storage-manager.db`, constructs the service, manages it as state, registers only `get_app_health`, and runs the generated context. `main.rs` calls `dev_storage_manager_lib::run()`.

Use typed APIs for the database path. Do not invoke a shell or accept a path from IPC.

- [ ] **Step 6: Verify the Tauri boundary**

Run:

```bash
cargo fmt --all --check
cargo test -p dev-storage-manager
cargo clippy -p dev-storage-manager --all-targets -- -D warnings
cargo check --workspace --all-targets
```

Expected: the focused command test and all checks pass; no destructive command appears in Tauri's generated command list.

- [ ] **Step 7: Commit the shell**

```bash
git add Cargo.toml Cargo.lock assets/app-icon.svg src-tauri
git commit -m "feat: add read-only Tauri shell"
```

---

### Task 6: React foundation screen and typed IPC client

**Files:**

- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/App.css`
- Create: `src/index.css`
- Create: `src/vite-env.d.ts`
- Create: `src/lib/desktop.ts`
- Create: `src/lib/desktop.test.ts`
- Create: `src/test/setup.ts`
- Create: `src/App.test.tsx`

**Interfaces:**

- Consumes: Tauri `invoke('get_app_health')` and the camel-case `AppHealth` payload.
- Produces: `getAppHealth(invokeFn?) -> Promise<AppHealth>` and a foundation-stage screen with loading, success, and recoverable error states.

- [ ] **Step 1: Write a failing typed-client test**

In `src/lib/desktop.test.ts`, write:

```ts
import { describe, expect, it, vi } from 'vitest';
import { getAppHealth } from './desktop';

describe('getAppHealth', () => {
  it('invokes only the read-only health command', async () => {
    const invoke = vi.fn().mockResolvedValue({
      appName: 'dev-storage-manager',
      appVersion: '0.1.0',
      platform: 'macos',
      databaseSchemaVersion: 1,
    });

    await expect(getAppHealth(invoke)).resolves.toMatchObject({
      platform: 'macos',
      databaseSchemaVersion: 1,
    });
    expect(invoke).toHaveBeenCalledWith('get_app_health');
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run the client test and confirm the expected failure**

Run:

```bash
npm run test:run -- src/lib/desktop.test.ts
```

Expected: the test fails because `desktop.ts` is absent.

- [ ] **Step 3: Implement the typed client**

Define:

```ts
import { invoke } from '@tauri-apps/api/core';

export interface AppHealth {
  appName: string;
  appVersion: string;
  platform: 'macos';
  databaseSchemaVersion: number;
}

type Invoke = <T>(command: string) => Promise<T>;

export function getAppHealth(invokeFn: Invoke = invoke): Promise<AppHealth> {
  return invokeFn<AppHealth>('get_app_health');
}
```

Run the focused test again and expect it to pass.

- [ ] **Step 4: Write failing UI behavior tests**

Configure `src/test/setup.ts` to import `@testing-library/jest-dom/vitest`. In `src/App.test.tsx`, mock `getAppHealth` and cover:

```tsx
it('describes the product as foundation-stage developer storage intelligence', async () => {
  render(<App />);
  expect(screen.getByRole('heading', { name: /understand what your development environment/i })).toBeVisible();
  expect(await screen.findByText(/foundation connected/i)).toBeVisible();
  expect(screen.getByText(/no automatic deletion/i)).toBeVisible();
});

it('shows a recoverable message when the desktop core is unavailable', async () => {
  vi.mocked(getAppHealth).mockRejectedValueOnce(new Error('offline'));
  render(<App />);
  expect(await screen.findByText(/desktop core is unavailable/i)).toBeVisible();
});
```

- [ ] **Step 5: Run UI tests and confirm the expected failure**

Run:

```bash
npm run test:run -- src/App.test.tsx
```

Expected: tests fail because the application component is absent.

- [ ] **Step 6: Implement the minimal accessible foundation screen**

Create semantic `main`, heading, status region, and three brief commitments: local-first, no automatic deletion, and macOS-first with Windows planned. On mount, call `getAppHealth`; render loading with `aria-live="polite"`, render `Foundation connected · schema 1` on success, and render `Desktop core is unavailable. Restart the app and try again.` on failure.

Keep styling local in `App.css` and base tokens in `index.css`. Support light/dark color schemes, visible focus, 200% text zoom, reduced motion, and a 760-pixel minimum-window layout. Do not show scan buttons, cleanup controls, dashboard data, fake screenshots, or implemented-feature claims.

- [ ] **Step 7: Verify frontend behavior and production output**

Run:

```bash
npm run test:run
npm run lint
npm run format:check
npm run build
```

Expected: tests, type-checking, linting, formatting, and Vite production build pass; `dist/` remains ignored.

- [ ] **Step 8: Commit the frontend**

```bash
git add index.html src
git commit -m "feat: add foundation status interface"
```

---

### Task 7: CI, full verification, and foundation documentation alignment

**Files:**

- Create: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `docs/architecture.md`
- Modify: `docs/roadmap.md`

**Interfaces:**

- Consumes: committed npm and Cargo lockfiles plus all repository quality scripts.
- Produces: macOS CI parity with local verification and accurate contributor setup instructions.

- [ ] **Step 1: Add the macOS CI workflow**

Create `.github/workflows/ci.yml` triggered on pull requests and pushes. Grant `contents: read`, set `concurrency` to cancel superseded runs on the same ref, and use `macos-latest` because Phase 01 supports macOS only.

Use these steps in order:

```yaml
- uses: actions/checkout@v6
- uses: actions/setup-node@v6
  with:
    node-version-file: .nvmrc
    cache: npm
- run: rustup update stable
- run: rustup component add rustfmt clippy
- run: npm ci
- run: npm run format:check
- run: npm run lint
- run: npm run test:run
- run: npm run build
- run: cargo fmt --all --check
- run: cargo clippy --workspace --all-targets --locked -- -D warnings
- run: cargo test --workspace --locked
- run: npm run tauri build -- --debug
```

Do not add release, signing, notarization, upload, telemetry, secret, or Windows jobs.

- [ ] **Step 2: Update human-facing development instructions**

Update `README.md` to say Phase 01 provides only a foundation shell and health check. Add exact local commands:

```bash
nvm use
npm ci
npm run tauri dev
```

Update `CONTRIBUTING.md` with the same prerequisites and the complete verification sequence. Update the Phase 01 boundary in `docs/architecture.md` and mark Phase 01 complete in `docs/roadmap.md` only after the full verification step succeeds. Do not describe scanning, providers, cleanup, or monitoring as implemented.

- [ ] **Step 3: Run focused and full verification**

Run:

```bash
npm run test:run -- src/lib/desktop.test.ts src/App.test.tsx
cargo test -p dev-storage-core
cargo test -p dev-storage-platform
cargo test -p dev-storage-persistence
cargo test -p dev-storage-application
cargo test -p dev-storage-manager
npm run format:check
npm run lint
npm run test:run
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run tauri build -- --debug
git diff --check
```

Expected: every command exits zero; the Tauri debug application bundle is produced locally; no signed or notarized artifact is claimed.

- [ ] **Step 4: Audit the Phase 01 boundary**

Run:

```bash
rg -n -i "delete|remove_dir|remove_file|trash|prune|uninstall|full disk access|windows|telemetry|fetch\(|axios|reqwest|~/Repositories" --glob '!docs/**' --glob '!package-lock.json' --glob '!Cargo.lock' .
rg -n "invoke_handler" src-tauri/src
git status --short
git diff --stat HEAD~1
```

Expected: the first search has no executable destructive, network, Windows, telemetry, assumed-workspace, or permission implementation; explanatory UI copy may contain "no automatic deletion." The command registry contains only `get_app_health`. Status contains only intended Task 7 files.

- [ ] **Step 5: Commit the verified foundation**

```bash
git add .github/workflows/ci.yml README.md CONTRIBUTING.md docs/architecture.md docs/roadmap.md
git commit -m "ci: verify application foundation"
```

- [ ] **Step 6: Report the implementation outcome without pushing**

Run:

```bash
git status --short
git log --oneline --decorate -7
git remote -v
```

Expected: the working tree is clean, scoped local commits are visible, and the pre-existing remote is unchanged. Do not push; provide the final local commit hash for human review.

---

## Phase 01 completion criteria

- The Tauri 2 application launches on macOS and renders the foundation screen.
- `get_app_health` is the only IPC command and crosses UI, Tauri, application, platform, and persistence boundaries.
- SQLite migration tests pass on memory and temporary-file databases.
- Frontend tests cover success and failure states without claiming product features.
- Rust tests and lints pass per crate and across the workspace.
- Frontend format, lint, test, type-check, and production build pass.
- A local debug Tauri bundle builds without signing or notarization.
- GitHub Actions runs the same checks on macOS with read-only repository permission.
- No later-phase behavior, default networking, telemetry, destructive operation, or assumed workspace path is present.
- Changes remain local until a human chooses to push them.
