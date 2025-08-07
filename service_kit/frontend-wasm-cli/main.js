import init, { init_cli, run_command } from '/cli-ui/forge_cli_wasm.js';

async function main() {
    // 1. Initialize xterm.js
    const term = new Terminal({
        cursorBlink: true,
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
        },
        cols: 120, // Set a reasonable terminal width
        scrollback: 1000,
        convertEol: true, // Convert \n to \r\n for proper line endings
    });
    const fitAddon = new FitAddon.FitAddon();
    term.loadAddon(fitAddon);
    term.open(document.getElementById('terminal'));
    fitAddon.fit();
    window.addEventListener('resize', () => fitAddon.fit());

    term.writeln('Welcome to the Forge CLI (WASM Interface)');
    term.writeln('------------------------------------------');
    term.writeln('');

    try {
        // 2. Load and initialize the WASM module
        term.write('Loading WASM module...');
        await init();
        term.writeln('\r✅ WASM module loaded successfully.');

        // 3. Fetch OpenAPI spec and initialize the CLI
        term.write('Fetching OpenAPI spec from http://localhost:3000/api-docs/openapi.json...');
        const response = await fetch('http://localhost:3000/api-docs/openapi.json');
        if (!response.ok) {
            throw new Error(`Failed to fetch spec: ${response.statusText}`);
        }
        const specJson = await response.text();
        
        init_cli(specJson);
        term.writeln('\r✅ CLI initialized with OpenAPI spec.');

    } catch (e) {
        term.writeln(`\r\n❌ Error during initialization: ${e}`);
        return;
    }

    // 4. Implement the REPL
    let currentLine = '';
    const prompt = '\r\n$ ';
    term.write(prompt);

    term.onKey(({ key, domEvent }) => {
        if (domEvent.keyCode === 13) { // Enter
            if (currentLine) {
                term.writeln(''); // Move to the next line
                // Execute the command through WASM
                const result = run_command(currentLine);
                // Clean ANSI escape sequences more thoroughly
                const cleanResult = result
                    .replace(/\x1b\[[0-9;]*m/g, '') // Remove color codes
                    .replace(/\x1b\[[0-9]*[A-Za-z]/g, '') // Remove other escape sequences
                    .replace(/\[\d+m/g, ''); // Remove remaining bracket sequences
                
                // Split by lines and write each line separately for better formatting
                const lines = cleanResult.split('\n');
                lines.forEach((line, index) => {
                    if (index === lines.length - 1 && line === '') {
                        // Skip empty last line to avoid extra newline
                        return;
                    }
                    term.writeln(line);
                });
                currentLine = '';
            }
            term.write(prompt);
        } else if (domEvent.keyCode === 8) { // Backspace
            if (currentLine.length > 0) {
                term.write('\b \b');
                currentLine = currentLine.slice(0, -1);
            }
        } else if (!domEvent.altKey && !domEvent.ctrlKey && !domEvent.metaKey) {
            currentLine += key;
            term.write(key);
        }
    });
}

main();
