export default async function globalTeardown() {
        for (const key of ['E2E_SERVER_PID', 'E2E_MOCK_PID']) {
                const pid = process.env[key];
                if (pid) {
                        try {
                                process.kill(Number(pid), 'SIGTERM');
                        } catch {
                                // already gone
                        }
                }
        }
}
