# NearbyShare

局域网内共享剪切板和数据文件的桌面软件

## 功能特性

- **设备发现**: 基于 mDNS 协议在局域网内自动发现其他设备
- **Token 认证**: 使用 Token 进行设备间的身份认证，确保安全性
- **剪切板共享**: 在已连接的设备之间共享剪切板内容
- **文件传输**: 支持设备间的文件传输
- **多设备连接**: 支持同时连接多台电脑

## 技术栈

- **前端**: React + TypeScript
- **后端**: Tauri 2 + Rust
- **网络**: mDNS (mdns-sd), TCP
- **认证**: SHA256 Token 哈希验证

## 开发环境

### 先决条件

1. [Node.js](https://nodejs.org/) (v18+)
2. [Rust](https://www.rust-lang.org/) (stable)
3. [Tauri CLI](https://tauri.app/guides/getting-started/prerequisites/)

### Linux 额外依赖

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev libgtk-3-dev libglib2.0-dev
```

### 安装依赖

```bash
npm install
```

### 开发模式运行

```bash
npm run tauri dev
```

### 构建应用

```bash
npm run tauri build
```

## 使用说明

1. **设置认证 Token**: 在所有需要连接的设备上设置相同的认证 Token
2. **启动服务**: 点击 "启动服务" 按钮开始接受连接
3. **搜索设备**: 点击 "搜索设备" 按钮发现局域网内的其他设备
4. **连接设备**: 在发现的设备列表中点击 "连接" 按钮
5. **共享剪切板**: 在 "剪切板" 页面读取本地剪切板并共享给已连接设备
6. **文件传输**: 在 "设备" 页面选择已连接的设备，点击 "发送文件" 按钮

## 项目结构

```
nearbyshare/
├── src/                    # React 前端源码
│   ├── App.tsx            # 主应用组件
│   ├── App.css            # 样式文件
│   └── main.tsx           # 入口文件
├── src-tauri/             # Tauri/Rust 后端源码
│   ├── src/
│   │   ├── lib.rs         # 主库入口
│   │   ├── auth.rs        # 认证模块
│   │   ├── discovery.rs   # mDNS 设备发现
│   │   ├── network.rs     # 网络通信
│   │   ├── clipboard.rs   # 剪切板共享
│   │   ├── file_transfer.rs # 文件传输
│   │   └── state.rs       # 应用状态管理
│   └── Cargo.toml         # Rust 依赖配置
├── package.json           # Node.js 依赖配置
└── README.md
```

## 许可证

MIT License
