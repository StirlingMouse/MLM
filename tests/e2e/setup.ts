import { execSync, spawn } from 'child_process';
import { existsSync, readdirSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(__dirname, '../..');
const DB_PATH = resolve(ROOT, 'test/e2e.db');
const CONFIG_PATH = resolve(ROOT, 'test/e2e-config.toml');
const SERVER_BIN = resolve(ROOT, 'target/debug/mlm');
const SETUP_BIN = resolve(ROOT, 'target/debug/create_test_db');
const MOCK_BIN = resolve(ROOT, 'target/debug/mock_server');
const WASM_DIR = resolve(ROOT, 'target/dx/mlm_web_dioxus/debug/web/public/wasm');
const SERVER_URL = 'http://localhost:3998';
const MOCK_URL = 'http://localhost:3997';

function wasmExists(): boolean {
        try {
                return readdirSync(WASM_DIR).some(f => f.endsWith('.wasm'));
        } catch {
                return false;
        }
}

async function waitForUrl(url: string, timeoutMs = 15_000): Promise<void> {
        const deadline = Date.now() + timeoutMs;
        while (Date.now() < deadline) {
                try {
                        const res = await fetch(url);
                        if (res.ok || res.status < 500) return;
                } catch {
                        // not ready yet
                }
                await new Promise(r => setTimeout(r, 300));
        }
        throw new Error(`${url} did not start within ${timeoutMs}ms`);
}

export default async function globalSetup() {
        // Build required binaries if not present
        if (!existsSync(SERVER_BIN) || !existsSync(SETUP_BIN) || !existsSync(MOCK_BIN)) {
                console.log('[e2e] Building binaries...');
                execSync('cargo build --bin mlm --bin create_test_db --bin mock_server', {
                        cwd: ROOT,
                        stdio: 'inherit',
                });
        }

        // Build WASM if not present
        if (!wasmExists()) {
                console.log('[e2e] Building WASM...');
                execSync('dx build --fullstack --skip-assets', {
                        cwd: resolve(ROOT, 'mlm_web_dioxus'),
                        stdio: 'inherit',
                        env: { ...process.env, PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` },
                });
        }

        // (Re)create isolated test database
        console.log('[e2e] Creating test database...');
        execSync(`"${SETUP_BIN}" "${DB_PATH}"`, { cwd: ROOT, stdio: 'inherit' });

        // Start mock server (MaM + qBittorrent APIs)
        console.log('[e2e] Starting mock server on port 3997...');
        const mock = spawn(MOCK_BIN, [], {
                cwd: ROOT,
                env: { ...process.env, MOCK_PORT: '3997', RUST_LOG: 'warn' },
                stdio: 'ignore',
                detached: false,
        });
        mock.on('error', err => { throw new Error(`Failed to start mock_server: ${err.message}`); });
        process.env.E2E_MOCK_PID = String(mock.pid);
        await waitForUrl(`${MOCK_URL}/api/v2/app/version`);

        // Start MLM server with test database and config
        console.log('[e2e] Starting server on port 3998...');
        const server = spawn(SERVER_BIN, [], {
                cwd: ROOT,
                env: {
                        ...process.env,
                        MLM_DB_FILE: DB_PATH,
                        MLM_CONFIG_FILE: CONFIG_PATH,
                        MLM_MAM_BASE_URL: MOCK_URL,
                        RUST_LOG: 'warn',
                },
                stdio: 'ignore',
                detached: false,
        });
        server.on('error', err => { throw new Error(`Failed to start server: ${err.message}`); });
        process.env.E2E_SERVER_PID = String(server.pid);

        await waitForUrl(`${SERVER_URL}/dioxus/torrents`);
        console.log('[e2e] Server ready.');
}

