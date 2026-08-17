// 墨斗 IDE - 前端应用（健壮版）
// 首先检查运行环境
(function() {
    'use strict';

    // 全局错误处理
    window.addEventListener('error', function(e) {
        console.error('全局错误:', e.message, e.filename, e.lineno);
        var statusEl = document.getElementById('status-message');
        if (statusEl) {
            statusEl.textContent = '错误: ' + e.message;
        }
    });

    // 检查 Tauri 环境
    var invoke = null;
    if (typeof window.__TAURI__ !== 'undefined' && window.__TAURI__.core) {
        invoke = window.__TAURI__.core.invoke;
        console.log('Tauri IPC 已就绪');
    } else {
        console.warn('Tauri IPC 不可用，使用模拟模式');
        invoke = function(cmd, args) {
            console.log('模拟 IPC 调用:', cmd, args);
            return Promise.resolve(null);
        };
    }

    // 应用状态
    var state = {
        projectRoot: null,
        fileTree: [],
        openTabs: [],
        activeTabIndex: -1,
        gitStatus: null,
        searchResults: [],
        selectedSearchIndex: 0,
        commandResults: [],
        selectedCommandIndex: 0,
       monacoEditor: null,
       monacoLoaded: false,
       xtermLoaded: false,
       legacyTerminalEnabled: false,
       restoring: false,
       maxTabs: 20,
       awaitingMaxTabs: false,
   };

   // DOM 元素缓存
   var elements = {};

   // ====================================================================
   // Dock 控制器（三方终端停靠）
   // ====================================================================
   var dock = {
       terminals: new Map(),
       activeId: null,
       _collapsed: true,
       _dockWidth: '38%',

       init: function() {
           var self = this;
           this.elDock = document.getElementById('terminal-dock');
           this.elSlot = document.getElementById('dock-slot');
           this.elTitle = document.getElementById('dock-title');
           this.elTabs = document.getElementById('dock-terminal-tabs');
           this.elNew = document.getElementById('btn-dock-new');
           this.elKill = document.getElementById('btn-dock-kill');
           this.elToggle = document.getElementById('btn-dock-toggle');
           this.elResize = document.getElementById('dock-resize-handle');

           // 默认折叠，避免挤压编辑器
           if (this.elDock) this.elDock.classList.add('collapsed');

           if (this.elNew) this.elNew.addEventListener('click', function() { self.create(); });
           if (this.elKill) this.elKill.addEventListener('click', function() { self.kill(); });
           if (this.elToggle) this.elToggle.addEventListener('click', function() { self.toggle(); });

           this.setupResizeHandle();
           this.render();
       },

       ensureExpanded: function() {
           if (this._collapsed) this.toggle();
       },

       // 新建一个内置终端（xterm.js + PTY），渲染在右侧面板内，与编辑器同一窗口
       create: function() {
           var self = this;
           this.ensureExpanded();
           var cwd = state.projectRoot || null;
           invoke('create_terminal', { shell: null, cwd: cwd }).then(function(info) {
               var termId = info.id;

               if (!state.xtermLoaded) {
                   self.elSlot.innerHTML = '<div style="padding:12px;color:#858585;font-family:monospace;">终端 #' + (termId + 1) + ' 已创建（xterm.js 未加载）</div>';
                   self.renderTabs();
                   return;
               }

               var term = new Terminal({
                   fontSize: 13,
                   fontFamily: '"SF Mono", "JetBrains Mono", Menlo, monospace',
                   theme: { background: '#1E1E1E', foreground: '#D4D4D4' },
                   cursorBlink: true,
                   scrollback: 10000,
                   macOptionIsMeta: true,
                   rightClickSelectsWord: true
               });
               var fitAddon = new FitAddon.FitAddon();
               term.loadAddon(fitAddon);

               var termContainer = document.createElement('div');
               termContainer.className = 'dock-term-container';
               self.elSlot.innerHTML = '';
               self.elSlot.appendChild(termContainer);
               term.open(termContainer);

               requestAnimationFrame(function() {
                   fitAddon.fit();
                   invoke('resize_terminal', { id: termId, cols: term.cols, rows: term.rows });
               });

               self.terminals.set(termId, { term: term, fitAddon: fitAddon, container: termContainer, poll: null, onResize: null });
               self.activeId = termId;
               self.renderTabs();

               term.onData(function(data) {
                   invoke('write_terminal', { id: termId, data: data });
               });

               var poll = setInterval(function() {
                   invoke('write_terminal', { id: termId, data: '' }).then(function(output) {
                       if (output) term.write(output);
                   });
               }, 16);

               var onResize = function() {
                   if (self.activeId !== termId) return;
                   fitAddon.fit();
                   invoke('resize_terminal', { id: termId, cols: term.cols, rows: term.rows });
               };
               window.addEventListener('resize', onResize);

               var rec = self.terminals.get(termId);
               if (rec) { rec.poll = poll; rec.onResize = onResize; }
           }).catch(function(e) {
               self.elSlot.innerHTML = '<div class="dock-placeholder"><p class="dock-error">创建终端失败: ' + escapeHtml(String(e)) + '</p></div>';
           });
       },

       // 兼容旧入口（命令面板「终端: 新建终端」与 ⌘` 快捷键）
       start: function() { this.create(); },

       switchTo: function(termId) {
           var rec = this.terminals.get(termId);
           if (!rec) return;
           this.activeId = termId;
           this.elSlot.innerHTML = '';
           this.elSlot.appendChild(rec.container);
           rec.fitAddon.fit();
           invoke('resize_terminal', { id: termId, cols: rec.term.cols, rows: rec.term.rows });
           this.renderTabs();
       },

       kill: function() {
            if (this.activeId === null || this.activeId === undefined) return;
            this.killTerminal(this.activeId);
        },

       killTerminal: function(termId) {
            var rec = this.terminals.get(termId);
            if (rec) {
                if (rec.poll) clearInterval(rec.poll);
                if (rec.onResize) window.removeEventListener('resize', rec.onResize);
                try { rec.term.dispose(); } catch (e) {}
                this.terminals.delete(termId);
            }
            invoke('close_terminal', { id: termId }).catch(function() {});
            if (this.activeId === termId) this.activeId = null;
            if (this.terminals.size > 0) {
                var firstId = this.terminals.keys().next().value;
                this.switchTo(firstId);
            } else {
                this.render();
            }
        },

       renderTabs: function() {
            if (!this.elTabs) return;
            var self = this;
            this.elTabs.innerHTML = '';
            this.terminals.forEach(function(rec, termId) {
                var tab = document.createElement('div');
                tab.className = 'dock-tab' + (termId === self.activeId ? ' active' : '');

                var label = document.createElement('span');
                label.className = 'dock-tab-label';
                label.textContent = 'zsh ' + (termId + 1);
                label.addEventListener('click', function() { self.switchTo(termId); });

                var close = document.createElement('span');
                close.className = 'dock-tab-close';
                close.textContent = '×';
                close.title = '关闭终端';
                close.addEventListener('click', function(e) {
                    e.stopPropagation();
                    self.killTerminal(termId);
                });

                tab.appendChild(label);
                tab.appendChild(close);
                self.elTabs.appendChild(tab);
            });
        },

       toggle: function() {
           var self = this;
           if (this._collapsed) {
               this._collapsed = false;
               if (this.elDock) {
                   this.elDock.classList.remove('collapsed');
                   this.elDock.style.setProperty('--dock-width', this._dockWidth);
               }
               if (this.elResize) this.elResize.classList.add('visible');
               // 展开后让活动终端重新 fit
               var rec = this.terminals.get(this.activeId);
               if (rec) {
                   setTimeout(function() {
                       rec.fitAddon.fit();
                       invoke('resize_terminal', { id: self.activeId, cols: rec.term.cols, rows: rec.term.rows });
                   }, 60);
               }
           } else {
               this._collapsed = true;
               if (this.elDock) this.elDock.classList.add('collapsed');
               if (this.elResize) this.elResize.classList.remove('visible');
           }
           this.render();
       },

       setupResizeHandle: function() {
           var self = this;
           if (!this.elResize) return;
           var dragging = false, startX = 0, startWidth = 0;
           this.elResize.addEventListener('mousedown', function(e) {
               dragging = true;
               startX = e.clientX;
               startWidth = self.elDock ? self.elDock.offsetWidth : 0;
               e.preventDefault();
           });
           document.addEventListener('mousemove', function(e) {
               if (!dragging) return;
               var delta = e.clientX - startX;
               var newWidth = startWidth + delta;
               var editorArea = document.querySelector('.editor-area');
               if (editorArea) {
                   var totalWidth = editorArea.offsetWidth;
                   var ratio = (newWidth / totalWidth) * 100;
                   var minRatio = (240 / totalWidth) * 100;
                   ratio = Math.max(minRatio, Math.min(60, ratio));
                   self._dockWidth = ratio + '%';
                   if (self.elDock) self.elDock.style.setProperty('--dock-width', self._dockWidth);
                   var rec = self.terminals.get(self.activeId);
                   if (rec) {
                       rec.fitAddon.fit();
                       invoke('resize_terminal', { id: self.activeId, cols: rec.term.cols, rows: rec.term.rows });
                   }
               }
           });
           document.addEventListener('mouseup', function() {
               dragging = false;
           });
       },

       render: function() {
           if (!this.elSlot) return;
           if (this.terminals.size === 0) {
               this.elSlot.innerHTML =
                   '<div class="dock-placeholder"><p>内置终端</p>' +
                   '<p style="font-size:12px;">终端与编辑器同窗口运行，不再弹出独立终端窗口</p>' +
                   '<button class="btn-primary" id="dock-ph-new">新建终端</button></div>';
               var phNew = document.getElementById('dock-ph-new');
               if (phNew) phNew.addEventListener('click', function() { dock.create(); });
           }
           if (this.elTitle) this.elTitle.textContent = '终端';
       }
   };

   // 保存当前会话（项目路径 + 打开的文件 + 活动文件）
   function saveSession() {
       if (state.restoring) return;
       var session = {
           projectRoot: state.projectRoot,
           openFiles: state.openTabs.map(function(t) { return t.path; }),
           activeFile: (state.activeTabIndex >= 0 && state.openTabs[state.activeTabIndex])
               ? state.openTabs[state.activeTabIndex].path : null
       };
       invoke('save_session', { session: session }).catch(function() {});
   }

   // 恢复上次会话（上次关闭时的项目与文件）
   function restoreSession() {
       invoke('load_session').then(function(session) {
           if (!session || !session.projectRoot) return;
           state.restoring = true;

           invoke('open_project', { path: session.projectRoot }).then(function(nodes) {
               state.projectRoot = session.projectRoot;
               state.fileTree = nodes || [];
               renderFileTree(state.fileTree);
               updateGitStatus();

               var files = session.openFiles || [];
               var chain = Promise.resolve();
               files.forEach(function(p) {
                   chain = chain.then(function() { return openFile(p); });
               });
               chain.then(function() {
                   state.restoring = false;
                   if (session.activeFile) {
                       var idx = state.openTabs.findIndex(function(t) { return t.path === session.activeFile; });
                       if (idx >= 0) switchTab(idx);
                   }
                   updateStatus('已恢复上次会话');
               }).catch(function(e) {
                   state.restoring = false;
                   console.error('恢复文件失败:', e);
               });
           }).catch(function(e) {
               state.restoring = false;
               console.error('恢复项目失败:', e);
               updateStatus('恢复上次项目失败');
           });
       }).catch(function(e) {
           console.error('加载会话失败:', e);
       });
   }

   // 初始化
   function init() {
       console.log('墨斗 IDE 初始化...');

       // 缓存 DOM 元素
       cacheElements();

       // 绑定基础事件（必须在任何情况下都执行）
       bindBasicEvents();

       // 读取最大标签页数量设置
       try {
           var savedMaxTabs = parseInt(localStorage.getItem('modou.maxTabs'), 10);
           if (!isNaN(savedMaxTabs) && savedMaxTabs >= 1) {
               state.maxTabs = Math.min(savedMaxTabs, 50);
           }
       } catch (e) {}

       // 点击其他地方时隐藏标签右键菜单
       document.addEventListener('click', hideTabContextMenu);
       document.addEventListener('contextmenu', hideTabContextMenu);

       // 初始化终端停靠
       dock.init();

       // 更新状态
       updateStatus('就绪');


        // 延迟初始化复杂组件
        setTimeout(function() {
            initMonaco();
            initXterm();
        }, 100);

        console.log('墨斗 IDE 初始化完成');
    }

    // 缓存 DOM 元素
    function cacheElements() {
        var ids = [
            'file-tree', 'tab-bar', 'editor-content', 'welcome-screen',
            'monaco-editor', 'status-message', 'git-branch', 'branch-name',
            'git-changes', 'git-status', 'cursor-position', 'editor-spaces',
            'editor-encoding', 'editor-eol', 'file-language',
            'search-overlay', 'search-input', 'search-results',
            'command-overlay', 'command-input', 'command-results'
        ];

        ids.forEach(function(id) {
            elements[id] = document.getElementById(id);
        });
    }

    // 绑定基础事件
    function bindBasicEvents() {
        console.log('绑定基础事件...');

        // 打开项目按钮
        var btnOpen = document.getElementById('btn-open-project');
        if (btnOpen) {
            btnOpen.addEventListener('click', openProject);
            console.log('绑定: 打开项目按钮');
        }

        // 活动栏切换（切换侧边栏面板）
        document.querySelectorAll('.activity-icon').forEach(function(icon) {
            icon.addEventListener('click', function() {
                document.querySelectorAll('.activity-icon').forEach(function(i) {
                    i.classList.remove('active');
                });
                icon.classList.add('active');
                switchSidebarPanel(icon.dataset.panel);
            });
        });

        // 设置面板：最大标签页数量（实时生效并持久化）
        var maxTabsInput = document.getElementById('setting-max-tabs');
        if (maxTabsInput) {
            maxTabsInput.addEventListener('input', function() {
                var n = parseInt(maxTabsInput.value, 10);
                if (!isNaN(n) && n >= 1) {
                    state.maxTabs = Math.min(n, 50);
                    saveMaxTabs();
                    updateStatus('最大标签页数量: ' + state.maxTabs);
                }
            });
        }

        // 刷新文件树
        var btnRefresh = document.getElementById('btn-refresh');
        if (btnRefresh) {
            btnRefresh.addEventListener('click', function() {
                if (state.projectRoot) {
                    updateStatus('正在刷新...');
                    invoke('open_project', { path: state.projectRoot }).then(function(nodes) {
                        state.fileTree = nodes || [];
                        renderFileTree(state.fileTree);
                        updateStatus('已刷新');
                    }).catch(function(e) {
                        updateStatus('刷新失败: ' + e);
                    });
                } else {
                    updateStatus('未打开文件夹');
                }
            });
        }

        // 全部折叠
        var btnCollapse = document.getElementById('btn-collapse');
        if (btnCollapse) {
            btnCollapse.addEventListener('click', function() {
                document.querySelectorAll('#file-tree .tree-item.expanded').forEach(function(el) {
                    el.classList.remove('expanded');
                    var chevron = el.querySelector('.chevron');
                    if (chevron) chevron.textContent = '▸';
                });
                document.querySelectorAll('#file-tree .tree-children.expanded').forEach(function(el) {
                    el.classList.remove('expanded');
                });
            });
        }

        // 搜索浮层
        if (elements['search-input']) {
            elements['search-input'].addEventListener('input', onSearchInput);
            elements['search-input'].addEventListener('keydown', onSearchKeydown);
        }
        if (elements['search-overlay']) {
            elements['search-overlay'].addEventListener('click', function(e) {
                if (e.target === elements['search-overlay'] || e.target.classList.contains('overlay-backdrop')) {
                    closeSearch();
                }
            });
        }

        // 命令面板
        if (elements['command-input']) {
            elements['command-input'].addEventListener('input', onCommandInput);
            elements['command-input'].addEventListener('keydown', onCommandKeydown);
        }
        if (elements['command-overlay']) {
            elements['command-overlay'].addEventListener('click', function(e) {
                if (e.target === elements['command-overlay'] || e.target.classList.contains('overlay-backdrop')) {
                    closeCommandPalette();
                }
            });
        }

        // 全局快捷键
        document.addEventListener('keydown', onGlobalKeydown);

        // 窗口拖动区域（macOS）
        setupWindowDrag();

        console.log('基础事件绑定完成');
    }

    // 切换侧边栏面板（资源管理器 / 搜索 / Git / 调试 / 设置）
    function switchSidebarPanel(panel) {
        var titles = {
            explorer: '资源管理器',
            search: '搜索',
            git: '源代码管理',
            debug: '运行和调试',
            settings: '设置'
        };
        var placeholders = {
            search: '全局搜索将在后续版本提供',
            debug: '运行和调试将在后续版本提供'
        };

        var titleEl = document.getElementById('sidebar-title');
        if (titleEl) titleEl.textContent = titles[panel] || '资源管理器';

        var fileTree = elements['file-tree'];
        var gitPanel = document.getElementById('sidebar-panel-git');
        var placeholderPanel = document.getElementById('sidebar-panel-placeholder');
        var settingsPanel = document.getElementById('sidebar-panel-settings');
        var actions = document.querySelector('.sidebar-actions');

        // 先全部隐藏
        if (fileTree) fileTree.style.display = 'none';
        if (gitPanel) gitPanel.style.display = 'none';
        if (placeholderPanel) placeholderPanel.style.display = 'none';
        if (settingsPanel) settingsPanel.style.display = 'none';
        if (actions) actions.style.display = 'none';

        if (panel === 'explorer') {
            if (fileTree) fileTree.style.display = 'block';
            if (actions) actions.style.display = 'flex';
        } else if (panel === 'git') {
            if (gitPanel) {
                gitPanel.style.display = 'block';
                renderGitPanel(gitPanel);
            }
        } else if (panel === 'settings') {
            if (settingsPanel) {
                settingsPanel.style.display = 'block';
                var maxTabsInput = document.getElementById('setting-max-tabs');
                if (maxTabsInput) maxTabsInput.value = String(state.maxTabs);
            }
        } else {
            if (placeholderPanel) {
                placeholderPanel.style.display = 'block';
                placeholderPanel.innerHTML = '<div class="empty-state"><p>' +
                    (placeholders[panel] || '该功能将在后续版本提供') + '</p></div>';
            }
        }
    }

    // 渲染 Git 面板（只读展示当前分支与变更文件）
    function renderGitPanel(container) {
        if (!state.projectRoot) {
            container.innerHTML = '<div class="empty-state"><p>未打开文件夹</p></div>';
            return;
        }
        container.innerHTML = '<div class="empty-state"><p>加载中...</p></div>';
        invoke('get_git_status').then(function(git) {
            var html = '<div style="padding:8px 12px;">' +
                '<div style="margin-bottom:12px;color:#cccccc;">分支: <strong>' + escapeHtml(git.branch) + '</strong></div>';
            if (git.modified && git.modified.length > 0) {
                html += '<div style="margin-bottom:4px;color:#858585;">已修改 (' + git.modified.length + ')</div>';
                git.modified.forEach(function(f) {
                    html += '<div class="tree-item" style="color:#e2c08d;">M ' + escapeHtml(f) + '</div>';
                });
            }
            if (git.added && git.added.length > 0) {
                html += '<div style="margin:8px 0 4px;color:#858585;">新增 (' + git.added.length + ')</div>';
                git.added.forEach(function(f) {
                    html += '<div class="tree-item" style="color:#73c991;">A ' + escapeHtml(f) + '</div>';
                });
            }
            if ((!git.modified || git.modified.length === 0) && (!git.added || git.added.length === 0)) {
                html += '<div style="color:#858585;">工作区干净，无变更</div>';
            }
            html += '</div>';
            container.innerHTML = html;
        }).catch(function() {
            container.innerHTML = '<div class="empty-state"><p>当前文件夹不是 Git 仓库</p></div>';
        });
    }

    function escapeHtml(s) {
        return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    // 设置窗口拖动
    function setupWindowDrag() {
        // 活动栏可拖动
        var activityBar = document.querySelector('.activity-bar');
        if (activityBar) {
            activityBar.style.webkitAppRegion = 'drag';
        }

        // 侧边栏头部可拖动（含顶部避让留白区）
        var sidebarHeader = document.querySelector('.sidebar-header');
        if (sidebarHeader) {
            sidebarHeader.style.webkitAppRegion = 'drag';
        }

        // 标签栏可拖动
        var tabBar = document.querySelector('.tab-bar');
        if (tabBar) {
            tabBar.style.webkitAppRegion = 'drag';
        }

        // 状态栏可拖动
        var statusBar = document.querySelector('.status-bar');
        if (statusBar) {
            statusBar.style.webkitAppRegion = 'drag';
        }

        // 但按钮不可拖动
        document.querySelectorAll('button, .btn-icon, .tab, .tree-item, input').forEach(function(el) {
            el.style.webkitAppRegion = 'no-drag';
        });
    }

    // 初始化 Monaco Editor
    function initMonaco() {
        if (typeof require === 'undefined') {
            console.warn('Monaco loader 不可用');
            showFallbackEditor();
            restoreSession();
            return;
        }

        try {
            require.config({ paths: { vs: 'lib/vs' } });
            require(['vs/editor/editor.main'], function() {
                console.log('Monaco Editor 加载成功');
                state.monacoLoaded = true;

                // 定义主题
                monaco.editor.defineTheme('modou-dark', {
                    base: 'vs-dark',
                    inherit: true,
                    rules: [
                        { token: 'comment', foreground: '6A9955' },
                        { token: 'keyword', foreground: '569CD6' },
                        { token: 'string', foreground: 'CE9178' },
                    ],
                    colors: {
                        'editor.background': '#1E1E1E',
                        'editor.foreground': '#D4D4D4',
                    }
                });

                // 创建编辑器（先清空容器，避免残留备用 <pre> 文本）
                if (elements['monaco-editor']) {
                    elements['monaco-editor'].innerHTML = '';
                    state.monacoEditor = monaco.editor.create(elements['monaco-editor'], {
                        value: '',
                        language: 'plaintext',
                        theme: 'modou-dark',
                        fontSize: 13,
                        fontFamily: '"SF Mono", "JetBrains Mono", Menlo, monospace',
                        lineNumbers: 'on',
                        minimap: { enabled: false },
                        automaticLayout: true,
                        tabSize: 4,
                        insertSpaces: true,
                        padding: { top: 8, bottom: 8 },
                    });

                    // 监听光标变化
                    state.monacoEditor.onDidChangeCursorPosition(function(e) {
                        var pos = e.position;
                        if (elements['cursor-position']) {
                            elements['cursor-position'].textContent = 'Ln ' + pos.lineNumber + ', Col ' + pos.column;
                        }
                    });

                    // 监听内容变化
                    state.monacoEditor.onDidChangeModelContent(function() {
                        if (state.activeTabIndex >= 0 && state.openTabs[state.activeTabIndex]) {
                            state.openTabs[state.activeTabIndex].isDirty = true;
                            state.openTabs[state.activeTabIndex].content = state.monacoEditor.getValue();
                            renderTabs();
                        }
                    });

                    // 若已有打开的标签（例如恢复了上次会话），切到 Monaco 渲染
                    if (state.activeTabIndex >= 0 && state.openTabs.length > 0) {
                        renderEditor();
                    }
                }

                // Monaco 就绪后再恢复会话，确保语法高亮正常
                restoreSession();
            }, function(err) {
                console.error('Monaco Editor 加载失败:', err);
                showFallbackEditor();
                restoreSession();
            });
        } catch (e) {
            console.error('Monaco 初始化异常:', e);
            showFallbackEditor();
            restoreSession();
        }
    }

    // 显示备用编辑器
    function showFallbackEditor() {
        if (elements['monaco-editor']) {
            elements['monaco-editor'].innerHTML = '<div style="padding:20px;color:#858585;font-family:monospace;">Monaco Editor 加载失败，使用只读模式</div>';
        }
    }

    // 初始化 xterm.js
    function initXterm() {
        if (typeof Terminal === 'undefined') {
            console.warn('xterm.js 不可用');
            return;
        }
        state.xtermLoaded = true;
        console.log('xterm.js 已就绪');
    }

    // 打开项目
    function openProject() {
        console.log('打开项目...');
        updateStatus('正在打开项目...');

        if (invoke) {
            invoke('pick_folder').then(function(path) {
                if (path) {
                    loadProject(path);
                } else {
                    // 用户取消了选择
                    updateStatus('就绪');
                }
            }).catch(function(e) {
                console.error('对话框失败:', e);
                fallbackOpenProject();
            });
        } else {
            fallbackOpenProject();
        }
    }

    function fallbackOpenProject() {
        // WKWebView 不支持 window.prompt，使用内联输入框
        var container = elements['file-tree'];
        container.innerHTML = '<div class="empty-state" style="padding:12px;">' +
            '<p style="margin-bottom:8px;">输入项目路径:</p>' +
            '<input id="project-path-input" type="text" placeholder="/Users/..." ' +
            'style="width:100%;padding:6px 8px;margin-bottom:8px;background:#3c3c3c;border:1px solid #555;color:#d4d4d4;border-radius:3px;outline:none;">' +
            '<button class="btn-primary" id="btn-confirm-path" style="width:100%;">打开</button></div>';
        var input = document.getElementById('project-path-input');
        var confirm = function() {
            var path = input.value.trim();
            if (path) {
                loadProject(path);
            }
        };
        document.getElementById('btn-confirm-path').addEventListener('click', confirm);
        input.addEventListener('keydown', function(e) {
            if (e.key === 'Enter') confirm();
        });
        input.focus();
    }

    // 加载项目
    function loadProject(path) {
        console.log('加载项目:', path);
        updateStatus('正在加载项目...');

        invoke('open_project', { path: path }).then(function(nodes) {
            state.projectRoot = path;
            state.fileTree = nodes || [];
            renderFileTree(state.fileTree);
            updateGitStatus();
            updateStatus('项目已打开');
            saveSession();
        }).catch(function(e) {
            console.error('加载项目失败:', e);
            updateStatus('打开项目失败: ' + e);
        });
    }

    // 渲染文件树
    function renderFileTree(nodes, container, depth) {
        container = container || elements['file-tree'];
        depth = depth || 0;

        if (depth === 0) {
            container.innerHTML = '';
        }

        if (!nodes || nodes.length === 0) {
            if (depth === 0) {
                container.innerHTML = '<div class="empty-state"><p>未打开文件夹</p><button class="btn-primary" id="btn-open-project">打开文件夹</button></div>';
                document.getElementById('btn-open-project').addEventListener('click', openProject);
            }
            return;
        }

        nodes.forEach(function(node) {
            var item = document.createElement('div');
            item.className = 'tree-item';
            item.style.paddingLeft = (8 + depth * 12) + 'px';

            if (node.is_dir) {
                var chevron = document.createElement('span');
                chevron.className = 'chevron';
                chevron.textContent = '▸';
                item.appendChild(chevron);

                var icon = document.createElement('span');
                icon.className = 'icon';
                icon.textContent = '📁';
                item.appendChild(icon);

                var name = document.createElement('span');
                name.textContent = node.name;
                item.appendChild(name);

                var childrenContainer = document.createElement('div');
                childrenContainer.className = 'tree-children';
                container.appendChild(item);
                container.appendChild(childrenContainer);

                // 已有子节点数据则直接渲染（懒加载前为空）
                if (node.children && node.children.length > 0) {
                    renderFileTree(node.children, childrenContainer, depth + 1);
                }

                item.addEventListener('click', function() {
                    var isExpanded = item.classList.contains('expanded');
                    item.classList.toggle('expanded');
                    childrenContainer.classList.toggle('expanded');
                    chevron.textContent = isExpanded ? '▸' : '▾';

                    // 首次展开时懒加载子目录
                    if (!isExpanded && !node.loaded) {
                        node.loaded = true;
                        invoke('list_dir', { path: node.path }).then(function(children) {
                            node.children = children || [];
                            renderFileTree(node.children, childrenContainer, depth + 1);
                        }).catch(function(e) {
                            console.error('加载目录失败:', e);
                            childrenContainer.innerHTML = '<div class="tree-item" style="color:#858585;">加载失败</div>';
                        });
                    }
                });
            } else {
                var icon = document.createElement('span');
                icon.className = 'icon';
                icon.textContent = getFileIcon(node.name);
                item.appendChild(icon);

                var name = document.createElement('span');
                name.textContent = node.name;
                item.appendChild(name);

                item.addEventListener('click', function() {
                    openFile(node.path);
                });
                container.appendChild(item);
            }
        });
    }

    // 获取文件图标
    function getFileIcon(name) {
        var ext = name.split('.').pop().toLowerCase();
        var icons = {
            'rs': '🦀', 'go': '🐹', 'py': '🐍',
            'ts': '📘', 'tsx': '📘', 'js': '📒', 'jsx': '📒',
            'md': '📝', 'json': '📋', 'toml': '⚙️',
            'yaml': '⚙️', 'yml': '⚙️',
            'html': '🌐', 'css': '🎨',
            'c': '⚡', 'cpp': '⚡', 'h': '⚡', 'hpp': '⚡',
        };
        return icons[ext] || '📄';
    }

    // 打开文件
    function openFile(path) {
        console.log('打开文件:', path);
        updateStatus('正在加载文件...');

        return invoke('read_file', { path: path }).then(function(file) {
            var existingIndex = state.openTabs.findIndex(function(t) { return t.path === path; });
            if (existingIndex >= 0) {
                switchTab(existingIndex);
                return;
            }

            var tab = {
                path: file.path,
                name: file.path.split('/').pop(),
                content: file.content,
                language: getLanguageId(file.path),
                isDirty: false,
            };

            // 超出最大标签数时，自动关闭最旧的未修改标签
            if (state.openTabs.length >= state.maxTabs) {
                var evictIndex = state.openTabs.findIndex(function(t) { return !t.isDirty; });
                if (evictIndex >= 0) closeTab(evictIndex);
            }

            state.openTabs.push(tab);
            state.activeTabIndex = state.openTabs.length - 1;

            renderTabs();
            renderEditor();
            updateStatus('文件已加载');
            saveSession();
        }).catch(function(e) {
            console.error('加载文件失败:', e);
            updateStatus('加载文件失败: ' + e);
        });
    }

    // 获取语言 ID（仅映射已内置的 Monaco 语言模块）
    function getLanguageId(filename) {
        var name = filename.split('/').pop().toLowerCase();
        // 无扩展名的特殊文件名
        if (name === 'dockerfile') return 'dockerfile';

        var ext = name.split('.').pop();
        var map = {
            'rs': 'rust', 'go': 'go',
            'py': 'python', 'pyi': 'python',
            'ts': 'typescript', 'tsx': 'typescript',
            'js': 'javascript', 'jsx': 'javascript', 'mjs': 'javascript', 'cjs': 'javascript',
            'md': 'markdown', 'json': 'json',
            'html': 'html', 'htm': 'html',
            'css': 'css', 'scss': 'scss', 'less': 'less',
            'xml': 'xml', 'svg': 'xml',
            'c': 'c', 'h': 'c',
            'cpp': 'cpp', 'cc': 'cpp', 'cxx': 'cpp', 'hpp': 'cpp',
            'm': 'objective-c', 'mm': 'objective-c',
            'cs': 'csharp', 'fs': 'fsharp', 'vb': 'vb',
            'java': 'java', 'kt': 'kotlin', 'kts': 'kotlin',
            'scala': 'scala', 'clj': 'clojure',
            'swift': 'swift', 'rb': 'ruby', 'php': 'php',
            'lua': 'lua', 'pl': 'perl', 'r': 'r', 'jl': 'julia',
            'dart': 'dart', 'ex': 'elixir', 'exs': 'elixir',
            'sql': 'sql', 'mysql': 'mysql', 'pgsql': 'pgsql',
            'sh': 'shell', 'bash': 'shell', 'zsh': 'shell',
            'ps1': 'powershell', 'bat': 'bat', 'cmd': 'bat',
            'ini': 'ini', 'cfg': 'ini',
            'dockerfile': 'dockerfile',
            'graphql': 'graphql', 'gql': 'graphql',
            'proto': 'protobuf', 'sol': 'solidity',
            'tf': 'hcl', 'hcl': 'hcl',
        };
        return map[ext] || 'plaintext';
    }

    // 渲染标签栏
    function renderTabs() {
        if (!elements['tab-bar']) return;
        elements['tab-bar'].innerHTML = '';

        state.openTabs.forEach(function(tab, index) {
            var tabEl = document.createElement('div');
            tabEl.className = 'tab' + (index === state.activeTabIndex ? ' active' : '');

            if (tab.isDirty) {
                var dot = document.createElement('span');
                dot.className = 'dirty-dot';
                dot.textContent = '●';
                tabEl.appendChild(dot);
            }

            var name = document.createElement('span');
            name.className = 'tab-name';
            name.textContent = tab.name;
            tabEl.appendChild(name);

            var closeBtn = document.createElement('button');
            closeBtn.className = 'close-btn';
            closeBtn.textContent = '×';
            closeBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                closeTab(index);
            });
            tabEl.appendChild(closeBtn);

            tabEl.addEventListener('click', function() {
                switchTab(index);
            });
            tabEl.addEventListener('contextmenu', function(e) {
                e.preventDefault();
                e.stopPropagation();
                showTabContextMenu(e.clientX, e.clientY, index);
            });
            elements['tab-bar'].appendChild(tabEl);
        });
    }

    // 切换标签
    function switchTab(index) {
        if (state.activeTabIndex >= 0 && state.monacoEditor && state.openTabs[state.activeTabIndex]) {
            state.openTabs[state.activeTabIndex].content = state.monacoEditor.getValue();
        }

        state.activeTabIndex = index;
        renderTabs();
        renderEditor();
        saveSession();
    }

    // 关闭标签
    function closeTab(index) {
        state.openTabs.splice(index, 1);
        if (state.activeTabIndex >= state.openTabs.length) {
            state.activeTabIndex = state.openTabs.length - 1;
        }
        if (state.activeTabIndex < 0) {
            if (elements['monaco-editor']) elements['monaco-editor'].style.display = 'none';
            if (elements['welcome-screen']) elements['welcome-screen'].style.display = 'flex';
        } else {
            renderEditor();
        }
        renderTabs();
        saveSession();
    }

    // 关闭除指定标签外的所有标签
    function closeOtherTabs(index) {
        var keep = state.openTabs[index];
        if (!keep) return;
        state.openTabs = [keep];
        state.activeTabIndex = 0;
        renderTabs();
        renderEditor();
        saveSession();
    }

    // 关闭所有标签
    function closeAllTabs() {
        state.openTabs = [];
        state.activeTabIndex = -1;
        if (elements['monaco-editor']) elements['monaco-editor'].style.display = 'none';
        if (elements['welcome-screen']) elements['welcome-screen'].style.display = 'flex';
        renderTabs();
        saveSession();
    }

    // 标签右键菜单
    var tabContextMenu = null;

    function hideTabContextMenu() {
        if (tabContextMenu) {
            tabContextMenu.remove();
            tabContextMenu = null;
        }
    }

    function showTabContextMenu(x, y, index) {
        hideTabContextMenu();
        tabContextMenu = document.createElement('div');
        tabContextMenu.className = 'tab-context-menu';

        var items = [
            { label: '关闭', action: function() { closeTab(index); } },
            { label: '关闭其他标签', action: function() { closeOtherTabs(index); } },
            { label: '关闭所有标签', action: closeAllTabs },
        ];
        items.forEach(function(it) {
            var item = document.createElement('div');
            item.className = 'tab-context-menu-item';
            item.textContent = it.label;
            item.addEventListener('click', function() {
                it.action();
                hideTabContextMenu();
            });
            tabContextMenu.appendChild(item);
        });

        document.body.appendChild(tabContextMenu);

        // 防止菜单超出窗口边界
        var rect = tabContextMenu.getBoundingClientRect();
        tabContextMenu.style.left = Math.min(x, window.innerWidth - rect.width - 4) + 'px';
        tabContextMenu.style.top = Math.min(y, window.innerHeight - rect.height - 4) + 'px';
    }

    // 渲染编辑器
    function renderEditor() {
        if (state.activeTabIndex < 0 || state.openTabs.length === 0) {
            if (elements['monaco-editor']) elements['monaco-editor'].style.display = 'none';
            if (elements['welcome-screen']) elements['welcome-screen'].style.display = 'flex';
            return;
        }

        var tab = state.openTabs[state.activeTabIndex];
        if (elements['welcome-screen']) elements['welcome-screen'].style.display = 'none';
        if (elements['monaco-editor']) elements['monaco-editor'].style.display = 'block';

        if (state.monacoEditor && state.monacoLoaded) {
            var model = monaco.editor.createModel(tab.content, tab.language);
            state.monacoEditor.setModel(model);
            state.monacoEditor.layout();
        } else if (elements['monaco-editor']) {
            // 备用显示
            var escaped = tab.content.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
            elements['monaco-editor'].innerHTML = '<pre style="padding:16px;color:#d4d4d4;font-family:monospace;font-size:13px;line-height:1.5;white-space:pre-wrap;overflow:auto;height:100%;">' + escaped + '</pre>';
        }

        // 更新状态栏
        if (elements['cursor-position']) elements['cursor-position'].style.display = 'flex';
        if (elements['editor-spaces']) elements['editor-spaces'].style.display = 'flex';
        if (elements['editor-encoding']) elements['editor-encoding'].style.display = 'flex';
        if (elements['editor-eol']) elements['editor-eol'].style.display = 'flex';
        if (elements['file-language']) {
            elements['file-language'].style.display = 'flex';
            elements['file-language'].textContent = tab.language;
        }
    }

    // 保存当前文件
    function saveCurrentFile() {
        if (state.activeTabIndex < 0) return;

        var tab = state.openTabs[state.activeTabIndex];
        var content = state.monacoEditor ? state.monacoEditor.getValue() : tab.content;

        invoke('save_file', { path: tab.path, content: content }).then(function() {
            tab.isDirty = false;
            tab.content = content;
            renderTabs();
            updateStatus('已保存');
        }).catch(function(e) {
            updateStatus('保存失败: ' + e);
        });
    }

    // 更新 Git 状态
    function updateGitStatus() {
        invoke('get_git_status').then(function(git) {
            state.gitStatus = git;
            if (elements['git-branch']) {
                elements['git-branch'].style.display = 'flex';
                elements['branch-name'].textContent = git.branch;
            }
            if (git.modified && git.modified.length > 0 || git.added && git.added.length > 0) {
                if (elements['git-status']) {
                    elements['git-status'].style.display = 'flex';
                    elements['git-changes'].textContent = (git.modified.length + git.added.length);
                }
            }
        }).catch(function(e) {
            if (elements['git-branch']) elements['git-branch'].style.display = 'none';
            if (elements['git-status']) elements['git-status'].style.display = 'none';
        });
    }

    // 搜索文件
    function openSearch() {
        if (elements['search-overlay']) {
            elements['search-overlay'].style.display = 'flex';
            elements['search-input'].focus();
            elements['search-input'].value = '';
            state.searchResults = [];
            state.selectedSearchIndex = 0;
            renderSearchResults();
        }
    }

    function closeSearch() {
        if (elements['search-overlay']) {
            elements['search-overlay'].style.display = 'none';
        }
    }

    function onSearchInput() {
        var query = elements['search-input'].value.toLowerCase();
        if (!query) {
            state.searchResults = [];
            renderSearchResults();
            return;
        }

        state.searchResults = [];
        searchInTree(state.fileTree, query, '');
        state.selectedSearchIndex = 0;
        renderSearchResults();
    }

    function searchInTree(nodes, query, prefix) {
        if (!nodes) return;
        nodes.forEach(function(node) {
            var path = prefix ? prefix + '/' + node.name : node.name;
            if (!node.is_dir && node.name.toLowerCase().includes(query)) {
                state.searchResults.push({
                    name: node.name,
                    path: path,
                    fullPath: node.path,
                });
            }
            if (node.children) {
                searchInTree(node.children, query, path);
            }
        });
    }

    function renderSearchResults() {
        if (!elements['search-results']) return;
        elements['search-results'].innerHTML = '';

        state.searchResults.slice(0, 10).forEach(function(result, index) {
            var item = document.createElement('div');
            item.className = 'search-result-item' + (index === state.selectedSearchIndex ? ' selected' : '');

            var icon = document.createElement('span');
            icon.className = 'file-icon';
            icon.textContent = getFileIcon(result.name);
            item.appendChild(icon);

            var name = document.createElement('span');
            name.textContent = result.name;
            item.appendChild(name);

            var path = document.createElement('span');
            path.className = 'file-path';
            path.textContent = result.path;
            item.appendChild(path);

            item.addEventListener('click', function() {
                openFile(result.fullPath);
                closeSearch();
            });

            elements['search-results'].appendChild(item);
        });
    }

    function onSearchKeydown(e) {
        if (e.key === 'Escape') closeSearch();
        else if (e.key === 'ArrowDown') {
            e.preventDefault();
            state.selectedSearchIndex = Math.min(state.selectedSearchIndex + 1, state.searchResults.length - 1);
            renderSearchResults();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            state.selectedSearchIndex = Math.max(state.selectedSearchIndex - 1, 0);
            renderSearchResults();
        } else if (e.key === 'Enter') {
            var result = state.searchResults[state.selectedSearchIndex];
            if (result) {
                openFile(result.fullPath);
                closeSearch();
            }
        }
    }

    // 命令面板
    var commands = [
        { name: '文件: 打开文件夹', action: openProject },
       { name: '文件: 保存文件', action: saveCurrentFile },
        { name: '终端: 新建终端', action: function() { dock.start(); } },
        { name: '转到: 转到文件', action: openSearch },
        { name: '标签: 关闭所有标签', action: closeAllTabs },
        { name: '设置: 最大标签页数量', action: beginMaxTabsInput },
    ];

    // 进入"最大标签页数量"输入模式（复用命令面板输入框）
    function beginMaxTabsInput() {
        state.awaitingMaxTabs = true;
        if (elements['command-overlay']) {
            elements['command-overlay'].style.display = 'flex';
            elements['command-input'].value = String(state.maxTabs);
            elements['command-input'].placeholder = '输入最大标签页数量（1-50），回车确认';
            elements['command-input'].focus();
            elements['command-input'].select();
        }
        state.commandResults = [];
        renderCommandResults();
    }

    function endMaxTabsInput() {
        state.awaitingMaxTabs = false;
        if (elements['command-input']) {
            elements['command-input'].placeholder = '输入命令...';
        }
    }

    function saveMaxTabs() {
        try {
            localStorage.setItem('modou.maxTabs', String(state.maxTabs));
        } catch (e) {}
    }

    function openCommandPalette() {
        if (elements['command-overlay']) {
            elements['command-overlay'].style.display = 'flex';
            elements['command-input'].focus();
            elements['command-input'].value = '';
            state.commandResults = commands;
            state.selectedCommandIndex = 0;
            renderCommandResults();
        }
    }

    function closeCommandPalette() {
        // "最大标签页数量"输入模式下保持面板打开
        if (state.awaitingMaxTabs) return;
        if (elements['command-overlay']) {
            elements['command-overlay'].style.display = 'none';
        }
    }

    function onCommandInput() {
        if (state.awaitingMaxTabs) {
            state.commandResults = [];
            renderCommandResults();
            return;
        }
        var query = elements['command-input'].value.toLowerCase();
        state.commandResults = commands.filter(function(c) {
            return c.name.toLowerCase().includes(query);
        });
        state.selectedCommandIndex = 0;
        renderCommandResults();
    }

    function renderCommandResults() {
        if (!elements['command-results']) return;
        elements['command-results'].innerHTML = '';

        state.commandResults.forEach(function(cmd, index) {
            var item = document.createElement('div');
            item.className = 'command-result-item' + (index === state.selectedCommandIndex ? ' selected' : '');
            item.textContent = cmd.name;
            item.addEventListener('click', function() {
                cmd.action();
                closeCommandPalette();
            });
            elements['command-results'].appendChild(item);
        });
    }

    function onCommandKeydown(e) {
        if (e.key === 'Escape') {
            endMaxTabsInput();
            closeCommandPalette();
        }
        else if (e.key === 'ArrowDown') {
            e.preventDefault();
            state.selectedCommandIndex = Math.min(state.selectedCommandIndex + 1, state.commandResults.length - 1);
            renderCommandResults();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            state.selectedCommandIndex = Math.max(state.selectedCommandIndex - 1, 0);
            renderCommandResults();
        } else if (e.key === 'Enter') {
            // "最大标签页数量"输入模式
            if (state.awaitingMaxTabs) {
                var n = parseInt(elements['command-input'].value, 10);
                if (!isNaN(n) && n >= 1) {
                    state.maxTabs = Math.min(n, 50);
                    saveMaxTabs();
                    updateStatus('最大标签页数量: ' + state.maxTabs);
                } else {
                    updateStatus('无效的数字，设置未修改');
                }
                endMaxTabsInput();
                closeCommandPalette();
                return;
            }
            var cmd = state.commandResults[state.selectedCommandIndex];
            if (cmd) {
                cmd.action();
                closeCommandPalette();
            }
        }
    }

    // 全局快捷键
    function onGlobalKeydown(e) {
        var isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
        var cmdKey = isMac ? e.metaKey : e.ctrlKey;

        if (cmdKey && e.key === 'o') {
            e.preventDefault();
            openProject();
        } else if (cmdKey && e.key === 'p' && !e.shiftKey) {
            e.preventDefault();
            openSearch();
        } else if (cmdKey && e.key === 'P' && e.shiftKey) {
            e.preventDefault();
            openCommandPalette();
       } else if (cmdKey && e.key === '`') {
           e.preventDefault();
            dock.start();
        } else if (cmdKey && e.key === 'w') {
            e.preventDefault();
            if (state.activeTabIndex >= 0) closeTab(state.activeTabIndex);
        } else if (cmdKey && e.key === 's') {
            e.preventDefault();
            saveCurrentFile();
        } else if (cmdKey && e.key === ',') {
            e.preventDefault();
            document.querySelectorAll('.activity-icon').forEach(function(i) {
                i.classList.remove('active');
            });
            var settingsIcon = document.querySelector('.activity-icon[data-panel="settings"]');
            if (settingsIcon) settingsIcon.classList.add('active');
            switchSidebarPanel('settings');
        }
    }

    // 更新状态栏
    function updateStatus(message) {
        if (elements['status-message']) {
            elements['status-message'].textContent = message;
        }
    }

    // 启动应用
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
