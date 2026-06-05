// ============================================================
// /web/vite.config.js
// V1.0 "滇池拂晓版" — Vite 构建配置
//
// 根据 Vite 官方文档的推荐，使用 defineConfig 函数
// 可以获得更优质的类型提示与智能补全。[7](@ref)[4](@ref)
// ============================================================

import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import path from 'path';

export default defineConfig(({ command, mode }) => {
  // 情境配置：根据命令或模式返回不同的配置[6](@ref)[7](@ref)
  // command === 'serve' 为开发环境，command === 'build' 为生产环境[5](@ref)
  
  const baseConfig = {
    // --- 项目根目录 ---
    // index.html 文件所在的位置，默认为项目根目录[8](@ref)[4](@ref)
    root: '.',

    // --- 公共基础路径 ---
    // 如果部署到子路径下，需要修改此配置[8](@ref)[4](@ref)
    base: '/',

    // --- 插件配置 ---
    // 用于扩展 Vite 的功能[4](@ref)
    plugins: [
      // vite-plugin-wasm：让 Vite 原生支持 WASM 模块加载
      // 无需手动处理 fetch 和 instantiate 流程
      wasm(),
      
      // vite-plugin-top-level-await：支持顶层 await
      // 简化 WASM 初始化代码，无需包裹 async 函数
      topLevelAwait(),
    ],

    // --- 解析配置 ---
    resolve: {
      // 路径别名：使用 @ 代替 src 目录[8](@ref)[4](@ref)
      alias: {
        '@': path.resolve(__dirname, './src'),
        '@wasm': path.resolve(__dirname, '../core/pkg'),
      },
      // 导入时可省略的文件扩展名列表[8](@ref)
      extensions: ['.mjs', '.js', '.ts', '.jsx', '.tsx', '.json'],
    },

    // --- 构建配置 ---
    build: {
      // 打包输出目录，默认是 dist[8](@ref)[4](@ref)
      outDir: 'dist',
      
      // 打包后静态资源存放的目录[4](@ref)
      assetsDir: 'assets',
      
      // 使用 esbuild 进行代码压缩，速度快且压缩率不错[8](@ref)[4](@ref)
      minify: 'esbuild',
      
      // 生成源映射文件，方便调试生产环境代码[8](@ref)
      sourcemap: false,
      
      // 将 WASM 文件作为独立资源处理，不内联到 JS 中
      rollupOptions: {
        output: {
          // 手动分包，将 WASM 相关代码单独打包
          manualChunks: {
            wasm: ['vite-plugin-wasm'],
          },
        },
      },
    },

    // --- 静态资源处理 ---
    // 指定额外的文件模式作为静态资源处理[6](@ref)
    assetsInclude: ['**/*.wasm'],

    // --- 开发服务器配置 ---
    server: {
      // 开发服务器端口号[8](@ref)[4](@ref)
      port: 5173,
      
      // 设为 true 时若端口已被占用则会直接退出，而不是尝试下一个可用端口[6](@ref)
      strictPort: false,
      
      // 服务器启动时自动在浏览器中打开应用程序[6](@ref)[4](@ref)
      open: true,
      
      // 代理配置（如果需要跨域请求）[6](@ref)[8](@ref)
      proxy: {
        // 示例：代理 /api 请求到本地后端
        // '/api': {
        //   target: 'http://localhost:3000',
        //   changeOrigin: true,
        //   rewrite: (path) => path.replace(/^\/api/, ''),
        // },
      },

      // 响应头配置[6](@ref)
      headers: {
        // 允许跨域加载 WASM 模块
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
      },
    },

    // --- 环境变量配置 ---
    // 定义全局常量替换方式，在代码中可以直接使用这些常量[8](@ref)[4](@ref)
    define: {
      __APP_VERSION__: JSON.stringify('1.0.0'),
      __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
    },
  };

  // 开发环境特有配置
  if (command === 'serve') {
    return {
      ...baseConfig,
      // 开发环境下可以启用一些调试功能
      build: {
        ...baseConfig.build,
        // 开发构建保留调试信息
        minify: false,
        sourcemap: true,
      },
    };
  }

  // 生产环境特有配置
  return {
    ...baseConfig,
    // 生产环境可以添加更多优化
    build: {
      ...baseConfig.build,
      // 压缩 WASM 文件
      rollupOptions: {
        ...baseConfig.build.rollupOptions,
        plugins: [],
      },
    },
  };
});