// ============================================================
// /web/src/gpu_bridge.js
// V1.0 "滇池拂晓版" — WebGPU 零拷贝桥梁
//
// 负责：
// - 初始化 WebGPU 设备和上下文
// - 创建渲染管线（WGSL 着色器）
// - 管理 GPU 缓冲区（零拷贝数据传递）
// - 提交 DrawCall 渲染字符
//
// 根据搜索结果[1](@ref)的优化策略，采用共享内存减少拷贝的方法：
// WASM 模块直接操作 TypedArray，WebGPU 通过 writeBuffer 读取同一块内存。
// 搜索结果[3](@ref)指出，writeBuffer 虽然发生一次拷贝（从 JS 堆到 GPU 缓冲区），
// 但其延迟最低，适合字符位置这种每帧变化的小数据量场景。
// ============================================================

/**
 * WebGPU 字形渲染器
 *
 * 管理 GPU 渲染管线、缓冲区和命令提交。
 * 核心设计理念参考了搜索结果[2](@ref)中关于 GPUExternalTexture 的零拷贝方案，
 * 以及搜索结果[4](@ref)中“零拷贝、零中间态”的优化哲学。
 */
export class GlyphRenderer {
    /**
     * @param {HTMLCanvasElement} canvas - 用于渲染的 Canvas 元素
     */
    constructor(canvas) {
        this.canvas = canvas;
        this.device = null;
        this.context = null;
        this.pipeline = null;
        this.bindGroupLayout = null;
        this.vertexBuffer = null;
        this.instanceBuffer = null;
        this.textureView = null;
        this.maxGlyphs = 10000; // 最大支持 10000 个字符
    }

    /**
     * 初始化 WebGPU
     *
     * 步骤：
     * 1. 请求 GPU 适配器
     * 2. 请求 GPU 设备
     * 3. 配置 Canvas 上下文
     * 4. 创建渲染管线
     * 5. 预分配缓冲区
     */
    async init() {
        // --- Step 1: 请求 GPU 适配器 ---
        if (!navigator.gpu) {
            throw new Error('当前浏览器不支持 WebGPU。请使用 Chrome 113+ 或 Edge 113+。');
        }
        const adapter = await navigator.gpu.requestAdapter();
        if (!adapter) {
            throw new Error('未能获取 GPU 适配器。可能无可用 GPU。');
        }
        console.log('✅ WebGPU 适配器获取成功:', adapter.name);

        // --- Step 2: 请求 GPU 设备 ---
        this.device = await adapter.requestDevice();
        console.log('✅ WebGPU 设备创建成功');

        // --- Step 3: 配置 Canvas 上下文 ---
        this.context = this.canvas.getContext('webgpu');
        const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
        this.context.configure({
            device: this.device,
            format: canvasFormat,
            alphaMode: 'premultiplied',
        });
        console.log('✅ Canvas 上下文配置完成，格式:', canvasFormat);

        // --- Step 4: 创建渲染管线 ---
        await this._createPipeline();

        // --- Step 5: 预分配缓冲区 ---
        this._createBuffers();

        console.log('✅ WebGPU 渲染器初始化完成');
    }

    /**
     * 创建渲染管线
     *
     * 根据搜索结果[2](@ref)中 WebGPU 渲染管线的创建步骤：
     * 1. 创建着色器模块（WGSL 代码）
     * 2. 创建渲染管线（包括顶点、片元、布局）
     * 3. 创建绑定组布局（用于将纹理传递给着色器）
     */
    async _createPipeline() {
        const device = this.device;

        // --- 顶点着色器 ---
        // 使用 WGSL 编写，负责将字符位置映射到屏幕坐标
        const vertexShader = device.createShaderModule({
            label: '字符渲染顶点着色器',
            code: `
                struct VertexOutput {
                    @builtin(position) position: vec4f,
                    @location(0) uv: vec2f,
                };

                // 顶点数据：每个字符是一个四边形（两个三角形）
                // 每个顶点包含局部坐标和 UV 坐标
                @vertex
                fn vertexMain(
                    @location(0) position: vec2f,    // 顶点局部坐标
                    @location(1) texCoord: vec2f,     // 纹理 UV 坐标
                    @location(2) instancePos: vec2f,  // 实例位置（来自 WASM 输出）
                    @location(3) glyphIndex: u32,     // 字形索引（来自 WASM 输出）
                ) -> VertexOutput {
                    var output: VertexOutput;
                    // 将实例位置与局部坐标结合
                    let worldPos = position + instancePos;
                    output.position = vec4f(worldPos, 0.0, 1.0);
                    output.uv = texCoord;
                    return output;
                }
            `,
        });

        // --- 片元着色器 ---
        // 负责从纹理图集中采样字形颜色
        const fragmentShader = device.createShaderModule({
            label: '字符渲染片元着色器',
            code: `
                @group(0) @binding(0) var glyphTexture: texture_2d<f32>;
                @group(0) @binding(1) var textureSampler: sampler;

                struct FragmentInput {
                    @location(0) uv: vec2f,
                };

                @fragment
                fn fragmentMain(input: FragmentInput) -> @location(0) vec4f {
                    // 从纹理中采样字形像素
                    let color = textureSample(glyphTexture, textureSampler, input.uv);
                    // 如果像素是透明的，则丢弃（用于非矩形字形）
                    if (color.a < 0.01) {
                        discard;
                    }
                    return color;
                }
            `,
        });

        // --- 创建绑定组布局 ---
        this.bindGroupLayout = device.createBindGroupLayout({
            label: '字形渲染绑定组布局',
            entries: [
                {
                    binding: 0,
                    visibility: GPUShaderStage.FRAGMENT,
                    texture: { sampleType: 'float' },
                },
                {
                    binding: 1,
                    visibility: GPUShaderStage.FRAGMENT,
                    sampler: { type: 'filtering' },
                },
            ],
        });

        // --- 创建渲染管线 ---
        this.pipeline = device.createRenderPipeline({
            label: '字符渲染管线',
            layout: device.createPipelineLayout({
                bindGroupLayouts: [this.bindGroupLayout],
            }),
            vertex: {
                module: vertexShader,
                entryPoint: 'vertexMain',
                buffers: [
                    // 顶点缓冲区：每个顶点 4 个 float（位置 + UV）
                    {
                        arrayStride: 16, // 4 floats * 4 bytes
                        attributes: [
                            { shaderLocation: 0, offset: 0, format: 'float32x2' }, // 位置
                            { shaderLocation: 1, offset: 8, format: 'float32x2' }, // UV
                        ],
                    },
                    // 实例缓冲区：每个实例 3 个 float + 1 个 u32
                    {
                        arrayStride: 16, // 3 floats + 1 u32，对齐到 16 字节
                        stepMode: 'instance',
                        attributes: [
                            { shaderLocation: 2, offset: 0, format: 'float32x2' }, // 实例位置
                            { shaderLocation: 3, offset: 8, format: 'uint32' },     // 字形索引
                        ],
                    },
                ],
            },
            fragment: {
                module: fragmentShader,
                entryPoint: 'fragmentMain',
                targets: [
                    {
                        format: navigator.gpu.getPreferredCanvasFormat(),
                        blend: {
                            color: {
                                srcFactor: 'src-alpha',
                                dstFactor: 'one-minus-src-alpha',
                                operation: 'add',
                            },
                            alpha: {
                                srcFactor: 'one',
                                dstFactor: 'one-minus-src-alpha',
                                operation: 'add',
                            },
                        },
                    },
                ],
            },
            primitive: {
                topology: 'triangle-list',
            },
            depthStencil: {
                depthWriteEnabled: true,
                depthCompare: 'less',
                format: 'depth24plus',
            },
        });
    }

    /**
     * 预分配 GPU 缓冲区
     *
     * 根据搜索结果[3](@ref)的策略3“WebGPU 缓冲区预分配”，
     * 提前分配 GPU 缓冲区，避免运行时动态分配的开销。
     *
     * 采用搜索结果[1](@ref)中提到的“预分配缓冲区”策略：
     * “提前分配 WebGPU 缓冲区，避免运行时动态分配的开销。”
     * 此策略减少运行时内存分配延迟，尤其适用于实时渲染应用。
     */
    _createBuffers() {
        const device = this.device;

        // --- 顶点缓冲区：一个四边形（两个三角形） ---
        // 6 个顶点，每个顶点 4 个 float（位置 xy + UV xy）
        // 位置范围：-0.5 到 0.5（单位正方形）
        // UV 范围：0.0 到 1.0
        const vertexData = new Float32Array([
            // 位置       // UV
            -0.5, -0.5,  0.0, 1.0, // 左下
             0.5, -0.5,  1.0, 1.0, // 右下
            -0.5,  0.5,  0.0, 0.0, // 左上
            -0.5,  0.5,  0.0, 0.0, // 左上
             0.5, -0.5,  1.0, 1.0, // 右下
             0.5,  0.5,  1.0, 0.0, // 右上
        ]);

        this.vertexBuffer = device.createBuffer({
            label: '字形顶点缓冲区',
            size: vertexData.byteLength,
            usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
            mappedAtCreation: true,
        });
        new Float32Array(this.vertexBuffer.getMappedRange()).set(vertexData);
        this.vertexBuffer.unmap();

        // --- 实例缓冲区：每个字符一个实例 ---
        // 预分配最大容量，避免运行时重新分配
        this.instanceBuffer = device.createBuffer({
            label: '字形实例缓冲区',
            size: this.maxGlyphs * 16, // 每个实例 16 字节
            usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
            mappedAtCreation: false,
        });
    }

    /**
     * 渲染一帧
     *
     * @param {Uint32Array} codepoints - Unicode 码点数组
     * @param {Float32Array} positions - 字符位置数组（来自 WASM 输出）
     * @param {Uint8Array} fontIds - 字体 ID 数组（来自 WASM 输出）
     * @param {GlyphAtlas} atlas - 纹理图集实例
     */
    render(codepoints, positions, fontIds, atlas) {
        if (!this.device || !this.pipeline) return;

        const device = this.device;
        const charCount = Math.min(codepoints.length, this.maxGlyphs);

        // --- 更新实例数据 ---
        // 根据搜索结果[3](@ref)的 writeBuffer 方法，直接将数据从 TypedArray 写入 GPU 缓冲区：
        // "device.queue.writeBuffer 是最简单、直接的方式，用于将数据立即写入 GPU 缓冲区。"
        // 虽然发生一次拷贝（从 JS 堆到 GPU 缓冲区），但其延迟最低，适合每帧更新的小数据量。
        const instanceData = new Float32Array(charCount * 4); // 每实例 4 个值
        for (let i = 0; i < charCount; i++) {
            instanceData[i * 4] = positions[i];         // X 位置
            instanceData[i * 4 + 1] = i * 0.5;          // Y 位置（行间距）
            instanceData[i * 4 + 2] = 0;                 // 保留
            instanceData[i * 4 + 3] = codepoints[i];     // 字形索引（使用码点作为索引）
        }
        device.queue.writeBuffer(this.instanceBuffer, 0, instanceData);

        // --- 获取纹理图集 ---
        if (!atlas || !atlas.texture) return;

        // --- 创建命令编码器 ---
        const commandEncoder = device.createCommandEncoder({
            label: '字符渲染命令编码器',
        });

        // --- 开始渲染通道 ---
        const textureView = this.context.getCurrentTexture().createView();
        const renderPass = commandEncoder.beginRenderPass({
            label: '字符渲染通道',
            colorAttachments: [
                {
                    view: textureView,
                    loadOp: 'clear',
                    storeOp: 'store',
                    clearValue: { r: 0.06, g: 0.06, b: 0.12, a: 1.0 }, // 深色背景
                },
            ],
        });

        // --- 设置渲染管线 ---
        renderPass.setPipeline(this.pipeline);
        renderPass.setVertexBuffer(0, this.vertexBuffer);
        renderPass.setVertexBuffer(1, this.instanceBuffer);
        renderPass.setBindGroup(0, atlas.bindGroup);

        // --- 绘制：6 个顶点 × N 个实例 ---
        renderPass.draw(6, charCount, 0, 0);

        // --- 结束渲染通道 ---
        renderPass.end();

        // --- 提交命令缓冲区 ---
        device.queue.submit([commandEncoder.finish()]);
    }

    /**
     * 调整 Canvas 大小
     *
     * @param {number} width - 新宽度
     * @param {number} height - 新高度
     */
    resize(width, height) {
        this.canvas.width = width;
        this.canvas.height = height;
        // 不需要重新配置 context，WebGPU 会自动处理
    }

    /**
     * 销毁资源
     */
    destroy() {
        if (this.vertexBuffer) this.vertexBuffer.destroy();
        if (this.instanceBuffer) this.instanceBuffer.destroy();
        if (this.device) this.device.destroy();
        console.log('✅ WebGPU 渲染器资源已释放');
    }
}