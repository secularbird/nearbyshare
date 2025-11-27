import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writeText, readText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import "./App.css";

interface Peer {
  id: string;
  name: string;
  ip: string;
  port: number;
  connected: boolean;
  authenticated: boolean;
}

interface FileTransfer {
  id: string;
  file_name: string;
  file_size: number;
  transferred: number;
  from_peer: string;
  to_peer: string;
  status: string;
}

type Tab = "peers" | "clipboard" | "files" | "settings";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("peers");
  const [token, setToken] = useState<string>("");
  const [customToken, setCustomToken] = useState<string>("");
  const [discoveredPeers, setDiscoveredPeers] = useState<Peer[]>([]);
  const [connectedPeers, setConnectedPeers] = useState<Peer[]>([]);
  const [clipboardContent, setClipboardContent] = useState<string>("");
  const [sharedClipboard, setSharedClipboard] = useState<string>("");
  const [pendingTransfers, setPendingTransfers] = useState<FileTransfer[]>([]);
  const [serverRunning, setServerRunning] = useState(false);
  const [discoveryRunning, setDiscoveryRunning] = useState(false);
  const [serverPort, setServerPort] = useState<number>(0);
  const [statusMessage, setStatusMessage] = useState<string>("");

  // Initialize app
  useEffect(() => {
    initApp();
    setupEventListeners();
  }, []);

  const initApp = async () => {
    try {
      const existingToken = await invoke<string | null>("get_token");
      if (existingToken) {
        setToken(existingToken);
      }
    } catch (e) {
      console.error("Failed to get token:", e);
    }
  };

  const setupEventListeners = async () => {
    // Listen for peer discovery events
    await listen<Peer>("peer-discovered", (event) => {
      setDiscoveredPeers((prev) => {
        const exists = prev.some((p) => p.id === event.payload.id);
        if (exists) return prev;
        return [...prev, event.payload];
      });
      setStatusMessage(`发现设备: ${event.payload.name}`);
    });

    await listen<string>("peer-removed", (event) => {
      setDiscoveredPeers((prev) => prev.filter((p) => p.id !== event.payload));
    });

    await listen<Peer>("peer-connected", (event) => {
      setConnectedPeers((prev) => [...prev, event.payload]);
      setStatusMessage(`设备已连接: ${event.payload.name}`);
    });

    await listen<string>("peer-disconnected", (event) => {
      setConnectedPeers((prev) => prev.filter((p) => p.id !== event.payload));
      setStatusMessage(`设备已断开`);
    });

    // Listen for clipboard events
    await listen<string>("clipboard-received", (event) => {
      setSharedClipboard(event.payload);
      setStatusMessage("收到剪切板内容");
    });

    // Listen for file transfer events
    await listen<FileTransfer>("file-transfer-request", (event) => {
      setPendingTransfers((prev) => [...prev, event.payload]);
      setStatusMessage(`收到文件传输请求: ${event.payload.file_name}`);
    });

    await listen<string>("file-transfer-complete", (event) => {
      setStatusMessage(`文件传输完成`);
      setPendingTransfers((prev) => prev.filter((t) => t.id !== event.payload));
    });
  };

  const generateToken = async () => {
    try {
      const newToken = await invoke<string>("generate_token");
      setToken(newToken);
      setStatusMessage("已生成新令牌");
    } catch (e) {
      console.error("Failed to generate token:", e);
      setStatusMessage("生成令牌失败");
    }
  };

  const saveCustomToken = async () => {
    if (!customToken.trim()) {
      setStatusMessage("请输入有效的令牌");
      return;
    }
    try {
      await invoke("set_token", { token: customToken });
      setToken(customToken);
      setCustomToken("");
      setStatusMessage("令牌已保存");
    } catch (e) {
      console.error("Failed to set token:", e);
      setStatusMessage("保存令牌失败");
    }
  };

  const startServer = async () => {
    try {
      const port = await invoke<number>("start_server");
      setServerPort(port);
      setServerRunning(true);
      await invoke("register_service");
      setStatusMessage(`服务器已启动，端口: ${port}`);
    } catch (e) {
      console.error("Failed to start server:", e);
      setStatusMessage("启动服务器失败");
    }
  };

  const stopServer = async () => {
    try {
      await invoke("stop_server");
      await invoke("unregister_service");
      setServerRunning(false);
      setStatusMessage("服务器已停止");
    } catch (e) {
      console.error("Failed to stop server:", e);
    }
  };

  const startDiscovery = async () => {
    try {
      await invoke("start_discovery");
      setDiscoveryRunning(true);
      setStatusMessage("开始搜索设备...");
    } catch (e) {
      console.error("Failed to start discovery:", e);
      setStatusMessage("搜索设备失败");
    }
  };

  const stopDiscovery = async () => {
    try {
      await invoke("stop_discovery");
      setDiscoveryRunning(false);
      setStatusMessage("已停止搜索");
    } catch (e) {
      console.error("Failed to stop discovery:", e);
    }
  };

  const connectToPeer = async (peerId: string) => {
    if (!token) {
      setStatusMessage("请先设置令牌");
      return;
    }
    try {
      const success = await invoke<boolean>("connect_to_peer", { peerId });
      if (success) {
        const peer = discoveredPeers.find((p) => p.id === peerId);
        if (peer) {
          setConnectedPeers((prev) => [...prev, { ...peer, connected: true, authenticated: true }]);
        }
        setStatusMessage("连接成功");
      } else {
        setStatusMessage("认证失败，请检查令牌");
      }
    } catch (e) {
      console.error("Failed to connect to peer:", e);
      setStatusMessage("连接失败");
    }
  };

  const disconnectFromPeer = async (peerId: string) => {
    try {
      await invoke("disconnect_from_peer", { peerId });
      setConnectedPeers((prev) => prev.filter((p) => p.id !== peerId));
      setStatusMessage("已断开连接");
    } catch (e) {
      console.error("Failed to disconnect:", e);
    }
  };

  const readLocalClipboard = async () => {
    try {
      const content = await readText();
      setClipboardContent(content || "");
    } catch (e) {
      console.error("Failed to read clipboard:", e);
    }
  };

  const shareClipboard = async () => {
    if (!clipboardContent) {
      setStatusMessage("剪切板为空");
      return;
    }
    try {
      await invoke("share_clipboard", { content: clipboardContent });
      setStatusMessage("剪切板已共享");
    } catch (e) {
      console.error("Failed to share clipboard:", e);
      setStatusMessage("共享失败");
    }
  };

  const copyToLocalClipboard = async () => {
    if (!sharedClipboard) {
      setStatusMessage("没有共享的剪切板内容");
      return;
    }
    try {
      await writeText(sharedClipboard);
      setStatusMessage("已复制到剪切板");
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
      setStatusMessage("复制失败");
    }
  };

  const selectAndSendFile = async (peerId: string) => {
    try {
      const filePath = await open({
        multiple: false,
        directory: false,
      });
      if (filePath) {
        await invoke("send_file", { peerId, filePath });
        setStatusMessage("文件发送请求已发送");
      }
    } catch (e) {
      console.error("Failed to send file:", e);
      setStatusMessage("文件发送失败");
    }
  };

  const acceptFileTransfer = async (transfer: FileTransfer) => {
    try {
      const savePath = await save({
        defaultPath: transfer.file_name,
      });
      if (savePath) {
        await invoke("accept_transfer", { transferId: transfer.id, savePath });
        setPendingTransfers((prev) => prev.filter((t) => t.id !== transfer.id));
        setStatusMessage("已接受文件传输");
      }
    } catch (e) {
      console.error("Failed to accept transfer:", e);
    }
  };

  const rejectFileTransfer = async (transferId: string) => {
    try {
      await invoke("reject_transfer", { transferId });
      setPendingTransfers((prev) => prev.filter((t) => t.id !== transferId));
      setStatusMessage("已拒绝文件传输");
    } catch (e) {
      console.error("Failed to reject transfer:", e);
    }
  };

  const refreshPeers = async () => {
    try {
      const peers = await invoke<Peer[]>("get_peers");
      setDiscoveredPeers(peers);
      const connected = await invoke<Peer[]>("get_connected_peers");
      setConnectedPeers(connected);
    } catch (e) {
      console.error("Failed to refresh peers:", e);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  return (
    <main className="app-container">
      <header className="app-header">
        <h1>NearbyShare</h1>
        <p className="subtitle">局域网共享</p>
      </header>

      <nav className="tab-nav">
        <button
          className={`tab-button ${activeTab === "peers" ? "active" : ""}`}
          onClick={() => setActiveTab("peers")}
        >
          📡 设备
        </button>
        <button
          className={`tab-button ${activeTab === "clipboard" ? "active" : ""}`}
          onClick={() => setActiveTab("clipboard")}
        >
          📋 剪切板
        </button>
        <button
          className={`tab-button ${activeTab === "files" ? "active" : ""}`}
          onClick={() => setActiveTab("files")}
        >
          📁 文件
        </button>
        <button
          className={`tab-button ${activeTab === "settings" ? "active" : ""}`}
          onClick={() => setActiveTab("settings")}
        >
          ⚙️ 设置
        </button>
      </nav>

      <div className="tab-content">
        {activeTab === "peers" && (
          <div className="panel">
            <div className="control-row">
              <button
                className={`control-button ${serverRunning ? "active" : ""}`}
                onClick={serverRunning ? stopServer : startServer}
              >
                {serverRunning ? "🔴 停止服务" : "🟢 启动服务"}
              </button>
              <button
                className={`control-button ${discoveryRunning ? "active" : ""}`}
                onClick={discoveryRunning ? stopDiscovery : startDiscovery}
              >
                {discoveryRunning ? "⏹️ 停止搜索" : "🔍 搜索设备"}
              </button>
              <button className="control-button" onClick={refreshPeers}>
                🔄 刷新
              </button>
            </div>

            {serverRunning && (
              <div className="info-box">
                <p>服务端口: <strong>{serverPort}</strong></p>
              </div>
            )}

            <h3>发现的设备 ({discoveredPeers.length})</h3>
            <div className="peer-list">
              {discoveredPeers.length === 0 ? (
                <p className="empty-message">暂无发现的设备</p>
              ) : (
                discoveredPeers.map((peer) => (
                  <div key={peer.id} className="peer-card">
                    <div className="peer-info">
                      <span className="peer-name">💻 {peer.name}</span>
                      <span className="peer-ip">{peer.ip}:{peer.port}</span>
                    </div>
                    <div className="peer-actions">
                      {connectedPeers.some((p) => p.id === peer.id) ? (
                        <button
                          className="action-button disconnect"
                          onClick={() => disconnectFromPeer(peer.id)}
                        >
                          断开
                        </button>
                      ) : (
                        <button
                          className="action-button connect"
                          onClick={() => connectToPeer(peer.id)}
                        >
                          连接
                        </button>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>

            <h3>已连接设备 ({connectedPeers.length})</h3>
            <div className="peer-list">
              {connectedPeers.length === 0 ? (
                <p className="empty-message">暂无连接的设备</p>
              ) : (
                connectedPeers.map((peer) => (
                  <div key={peer.id} className="peer-card connected">
                    <div className="peer-info">
                      <span className="peer-name">✅ {peer.name}</span>
                      <span className="peer-ip">{peer.ip}:{peer.port}</span>
                    </div>
                    <div className="peer-actions">
                      <button
                        className="action-button file"
                        onClick={() => selectAndSendFile(peer.id)}
                      >
                        发送文件
                      </button>
                      <button
                        className="action-button disconnect"
                        onClick={() => disconnectFromPeer(peer.id)}
                      >
                        断开
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {activeTab === "clipboard" && (
          <div className="panel">
            <div className="clipboard-section">
              <h3>本地剪切板</h3>
              <div className="control-row">
                <button className="control-button" onClick={readLocalClipboard}>
                  📖 读取剪切板
                </button>
                <button
                  className="control-button primary"
                  onClick={shareClipboard}
                  disabled={!clipboardContent}
                >
                  📤 共享
                </button>
              </div>
              <textarea
                className="clipboard-textarea"
                value={clipboardContent}
                onChange={(e) => setClipboardContent(e.target.value)}
                placeholder="点击'读取剪切板'获取内容，或直接输入要共享的内容"
              />
            </div>

            <div className="clipboard-section">
              <h3>收到的剪切板</h3>
              <div className="control-row">
                <button
                  className="control-button primary"
                  onClick={copyToLocalClipboard}
                  disabled={!sharedClipboard}
                >
                  📥 复制到剪切板
                </button>
              </div>
              <textarea
                className="clipboard-textarea"
                value={sharedClipboard}
                readOnly
                placeholder="等待接收共享的剪切板内容..."
              />
            </div>
          </div>
        )}

        {activeTab === "files" && (
          <div className="panel">
            <h3>待处理的文件传输 ({pendingTransfers.length})</h3>
            <div className="transfer-list">
              {pendingTransfers.length === 0 ? (
                <p className="empty-message">暂无待处理的文件传输</p>
              ) : (
                pendingTransfers.map((transfer) => (
                  <div key={transfer.id} className="transfer-card">
                    <div className="transfer-info">
                      <span className="transfer-name">📄 {transfer.file_name}</span>
                      <span className="transfer-size">{formatFileSize(transfer.file_size)}</span>
                      <span className="transfer-from">来自: {transfer.from_peer}</span>
                    </div>
                    <div className="transfer-actions">
                      <button
                        className="action-button accept"
                        onClick={() => acceptFileTransfer(transfer)}
                      >
                        接受
                      </button>
                      <button
                        className="action-button reject"
                        onClick={() => rejectFileTransfer(transfer.id)}
                      >
                        拒绝
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>

            <h3>发送文件</h3>
            <p className="hint">在"设备"页面选择已连接的设备，点击"发送文件"按钮</p>
          </div>
        )}

        {activeTab === "settings" && (
          <div className="panel">
            <h3>认证令牌</h3>
            <div className="token-section">
              {token ? (
                <div className="current-token">
                  <p>当前令牌:</p>
                  <code className="token-display">{token}</code>
                  <p className="hint">请确保所有要连接的设备使用相同的令牌</p>
                </div>
              ) : (
                <p className="empty-message">尚未设置令牌</p>
              )}
              
              <div className="control-row">
                <button className="control-button primary" onClick={generateToken}>
                  🎲 生成新令牌
                </button>
              </div>

              <div className="token-input-section">
                <p>或输入自定义令牌:</p>
                <div className="input-row">
                  <input
                    type="text"
                    className="token-input"
                    value={customToken}
                    onChange={(e) => setCustomToken(e.target.value)}
                    placeholder="输入令牌..."
                  />
                  <button className="control-button" onClick={saveCustomToken}>
                    保存
                  </button>
                </div>
              </div>
            </div>

            <h3>使用说明</h3>
            <div className="help-section">
              <ol>
                <li>在所有需要连接的设备上设置相同的认证令牌</li>
                <li>点击"启动服务"开始接受连接</li>
                <li>点击"搜索设备"发现局域网内的其他设备</li>
                <li>点击"连接"按钮连接到其他设备</li>
                <li>连接后即可共享剪切板和传输文件</li>
              </ol>
            </div>
          </div>
        )}
      </div>

      {statusMessage && (
        <footer className="status-bar">
          <span>{statusMessage}</span>
        </footer>
      )}
    </main>
  );
}

export default App;
