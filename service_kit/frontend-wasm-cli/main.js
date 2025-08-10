import init, { 
    init_cli, 
    run_command_async,
    get_completions,
    get_history_item,
    search_history
} from '/cli-ui/forge_cli_wasm.js';

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
        const baseUrl = window.location.origin; // 动态获取base URL
        term.write(`Fetching OpenAPI spec from ${baseUrl}/api-docs/openapi.json...`);
        const response = await fetch(`${baseUrl}/api-docs/openapi.json`);
        if (!response.ok) {
            throw new Error(`Failed to fetch spec: ${response.statusText}`);
        }
        const specJson = await response.text();
        const spec = JSON.parse(specJson);
        // 保存到全局以便 JS fallback 使用
        window.__openapiSpec = spec;
        window.__baseUrl = baseUrl;

        init_cli(specJson, baseUrl);
        term.writeln('\r✅ CLI initialized with OpenAPI spec.');

    } catch (e) {
        term.writeln(`\r\n❌ Error during initialization: ${e}`);
        return;
    }

    // 4. Implement the REPL with enhanced functionality
    let currentLine = '';
    let cursorPosition = 0; // 光标在当前行中的位置
    let historyIndex = -1; // -1表示当前输入，>=0表示历史记录索引
    let isInReverseSearch = false;
    let reverseSearchQuery = '';
    let completionMenu = null; // 当前显示的补全菜单
    
    const prompt = '\r\n$ ';
    const promptOnly = '$ '; // 不包含换行的提示符，用于重绘
    
    // 重绘当前行
    function redrawLine() {
        // 移动到行首并清除从提示符后的所有内容
        term.write('\r' + promptOnly);
        term.write('\x1b[K'); // 清除从光标到行尾的内容
        
        if (isInReverseSearch) {
            // 在反向搜索模式下，替换整个提示符
            term.write('\r\x1b[K'); // 清除整行
            term.write(`(reverse-i-search)'${reverseSearchQuery}': ${currentLine}`);
        } else {
            term.write(currentLine);
        }
        
        // 移动光标到正确位置
        if (cursorPosition < currentLine.length) {
            const moveCursor = currentLine.length - cursorPosition;
            term.write('\x1b[' + moveCursor + 'D'); // 向左移动光标
        }
    }
    
    // 插入字符到当前位置
    function insertChar(char) {
        currentLine = currentLine.slice(0, cursorPosition) + char + currentLine.slice(cursorPosition);
        cursorPosition++;
        redrawLine();
    }
    
    // 删除字符
    function deleteChar() {
        if (cursorPosition > 0) {
            currentLine = currentLine.slice(0, cursorPosition - 1) + currentLine.slice(cursorPosition);
            cursorPosition--;
            redrawLine();
        }
    }
    
    // 移动光标
    function moveCursor(direction) {
        if (direction === 'left' && cursorPosition > 0) {
            cursorPosition--;
            term.write('\x1b[D');
        } else if (direction === 'right' && cursorPosition < currentLine.length) {
            cursorPosition++;
            term.write('\x1b[C');
        }
    }
    
    // 处理Tab补全
    function handleTabCompletion() {
        try {
            const completionResult = get_completions(currentLine, cursorPosition);
            const suggestions = JSON.parse(completionResult.suggestions);
            
            if (suggestions.length === 0) {
                return;
            }
            
            if (suggestions.length === 1) {
                // 只有一个建议，直接补全
                const suggestion = suggestions[0];
                const beforeCursor = currentLine.slice(0, suggestion.start_pos);
                const afterCursor = currentLine.slice(suggestion.end_pos);
                currentLine = beforeCursor + suggestion.value + afterCursor;
                cursorPosition = beforeCursor.length + suggestion.value.length;
                redrawLine();
            } else {
                // 多个建议，显示补全菜单
                term.writeln('');
                suggestions.slice(0, 10).forEach(suggestion => {
                    const desc = suggestion.description ? ` - ${suggestion.description}` : '';
                    term.writeln(`  ${suggestion.value}${desc}`);
                });
                redrawLine();
            }
        } catch (e) {
            console.error('Tab completion error:', e);
        }
    }
    
    // 处理历史记录导航
    function navigateHistory(direction) {
        if (direction === 'up') {
            const item = get_history_item(historyIndex + 1);
            if (item) {
                historyIndex++;
                currentLine = item;
                cursorPosition = currentLine.length;
                redrawLine();
            }
        } else if (direction === 'down') {
            if (historyIndex > 0) {
                historyIndex--;
                const item = get_history_item(historyIndex);
                if (item) {
                    currentLine = item;
                    cursorPosition = currentLine.length;
                    redrawLine();
                }
            } else if (historyIndex === 0) {
                historyIndex = -1;
                currentLine = '';
                cursorPosition = 0;
                redrawLine();
            }
        }
    }
    
    // 处理反向搜索
    function handleReverseSearch(char) {
        if (char) {
            reverseSearchQuery += char;
        }
        
        try {
            const searchResults = JSON.parse(search_history(reverseSearchQuery));
            if (searchResults.length > 0) {
                currentLine = searchResults[0];
                cursorPosition = currentLine.length;
            }
            redrawLine();
        } catch (e) {
            console.error('Reverse search error:', e);
        }
    }
    
    // 退出反向搜索模式
    function exitReverseSearch() {
        isInReverseSearch = false;
        reverseSearchQuery = '';
        cursorPosition = currentLine.length;
        redrawLine();
    }
    
    // JS fallback：当 wasm 返回 Path not found 时，用 JS 直接按 OpenAPI 执行
    async function executeCommandJS(commandLine) {
        try {
            const spec = window.__openapiSpec;
            const baseUrl = window.__baseUrl || '';
            if (!spec) return 'Error: OpenAPI spec not loaded.';
            const tokens = commandLine.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
            if (tokens.length === 0) return '';
            const cmd = tokens[0];
            const args = {};
            for (let i = 1; i < tokens.length; i++) {
                const t = tokens[i];
                if (t.startsWith('--')) {
                    const key = t.replace(/^--/, '');
                    const val = (i + 1 < tokens.length && !tokens[i + 1].startsWith('--')) ? tokens[++i] : '';
                    args[key] = val.replace(/^"|"$/g, '');
                }
            }
            const parts = cmd.split('.');
            const method = parts.pop().toUpperCase();
            const cmdSegs = parts;
            // 匹配路径模板
            let matched = null;
            for (const [key, item] of Object.entries(spec.paths || {})) {
                const keySegs = key.split('/').filter(s => s);
                if (keySegs.length !== cmdSegs.length) continue;
                let ok = true;
                for (let i = 0; i < keySegs.length; i++) {
                    const ks = keySegs[i];
                    const cs = cmdSegs[i];
                    const isParam = ks.startsWith('{') && ks.endsWith('}');
                    if (!isParam && ks !== cs) { ok = false; break; }
                }
                if (ok) { matched = [key, item]; break; }
            }
            if (!matched) {
                return `API request failed (JS fallback): Path not found for /${cmdSegs.join('/')}`;
            }
            const [pathTemplate, pathItem] = matched;
            const op = (pathItem[method.toLowerCase()]);
            if (!op) return `API request failed (JS fallback): Operation not found for ${cmd}`;
            // 构造路径和查询
            let finalPath = pathTemplate;
            const used = new Set();
            if (Array.isArray(op.parameters)) {
                for (const p of op.parameters) {
                    const prm = p && p.name ? p : (p && p.$ref ? null : null);
                    if (!prm) continue;
                    if (p.in === 'path' && args[p.name] != null) {
                        finalPath = finalPath.replace(`{${p.name}}`, encodeURIComponent(args[p.name]));
                        used.add(p.name);
                    }
                }
            }
            const query = [];
            for (const [k, v] of Object.entries(args)) {
                if (!used.has(k)) query.push(`${encodeURIComponent(k)}=${encodeURIComponent(v)}`);
            }
            let serverUrl = '';
            if (Array.isArray(spec.servers) && spec.servers.length > 0 && spec.servers[0].url) {
                serverUrl = spec.servers[0].url;
            }
            const url = `${baseUrl}${serverUrl}${finalPath}${query.length ? ('?' + query.join('&')) : ''}`;
            const resp = await fetch(url, { method });
            const text = await resp.text();
            try {
                return JSON.stringify(JSON.parse(text), null, 2);
            } catch {
                return text;
            }
        } catch (e) {
            return `API request failed (JS fallback): ${e}`;
        }
    }

    term.write(prompt);

    term.onKey(({ key, domEvent }) => {
        const { keyCode, ctrlKey, altKey, metaKey } = domEvent;
        
        // Ctrl+R - 反向搜索
        if (ctrlKey && keyCode === 82 && !isInReverseSearch) {
            isInReverseSearch = true;
            reverseSearchQuery = '';
            currentLine = '';
            cursorPosition = 0;
            redrawLine();
            return;
        }
        
        // 在反向搜索模式下的处理
        if (isInReverseSearch) {
            if (keyCode === 13) { // Enter - 接受搜索结果
                exitReverseSearch();
                return;
            } else if (keyCode === 27) { // Esc - 取消搜索
                isInReverseSearch = false;
                reverseSearchQuery = '';
                currentLine = '';
                cursorPosition = 0;
                redrawLine();
                return;
            } else if (keyCode === 8) { // Backspace - 删除搜索字符
                if (reverseSearchQuery.length > 0) {
                    reverseSearchQuery = reverseSearchQuery.slice(0, -1);
                    handleReverseSearch();
                }
                return;
            } else if (!ctrlKey && !altKey && !metaKey && key.length === 1) {
                handleReverseSearch(key);
                return;
            }
            return;
        }
        
        // 普通模式下的处理
        if (keyCode === 13) { // Enter - 执行命令
            if (currentLine.trim()) {
                term.writeln('');
                
                // 异步执行命令
                (async () => {
                    try {
                        let result = await run_command_async(currentLine);
                        const plain = String(result);
                        if (plain.includes('Path not found for')) {
                            result = await executeCommandJS(currentLine);
                        }
                        // 清理ANSI转义序列
                        const cleanResult = String(result)
                            .replace(/\x1b\[[0-9;]*m/g, '')
                            .replace(/\x1b\[[0-9]*[A-Za-z]/g, '')
                            .replace(/\[\d+m/g, '');
                        
                        const lines = cleanResult.split('\n');
                        lines.forEach((line, index) => {
                            if (index === lines.length - 1 && line === '') {
                                return;
                            }
                            term.writeln(line);
                        });
                    } catch (error) {
                        term.writeln(`Error: ${error}`);
                    }
                    
                    term.write(prompt);
                })();
                
                currentLine = '';
                cursorPosition = 0;
                historyIndex = -1;
            } else {
                term.write(prompt);
            }
        } else if (keyCode === 9) { // Tab - 补全
            domEvent.preventDefault();
            handleTabCompletion();
        } else if (keyCode === 8) { // Backspace
            deleteChar();
        } else if (keyCode === 37) { // 左箭头
            moveCursor('left');
        } else if (keyCode === 39) { // 右箭头
            moveCursor('right');
        } else if (keyCode === 38) { // 上箭头 - 历史记录上一个
            navigateHistory('up');
        } else if (keyCode === 40) { // 下箭头 - 历史记录下一个
            navigateHistory('down');
        } else if (keyCode === 36) { // Home - 移到行首
            cursorPosition = 0;
            redrawLine();
        } else if (keyCode === 35) { // End - 移到行尾
            cursorPosition = currentLine.length;
            redrawLine();
        } else if (!ctrlKey && !altKey && !metaKey && key.length === 1) {
            // 普通字符输入
            insertChar(key);
        }
    });
}

main();
