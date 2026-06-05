// ============================================================
// /web/src/texture_atlas.js
// V1.0 "滇池拂晓版" — 纹理图集烘焙器
//
// 负责将字符字形烘焙到 WebGPU 纹理图集中，
// 实现一次 DrawCall 渲染万字符的关键技术。
//
// 根据搜索结果[1](@ref)中关于纹理图集的描述：
// "一次 DrawCall 渲染万字符，纹理图集烘焙"
// 这是我们的引擎能够实现 65KB 核心体积和极致性能的关键。
// ============================================================

/**
 * 纹理图集烘焙器
 *
 * 将多个字体的字形预先渲染到一张大纹理中，
 * 运行时只需一次纹理采样即可获取任何字符的字形。
 *
 * 设计参考了搜索结果[2](@ref)中关于 GPUExternalTexture 的零拷贝方案，
 * 以及经典图集算法（Atlas Algorithm）的节省空间策略。
 */
export class GlyphAtlas {
    /**
     * @param {GPUDevice} device - WebGPU 设备实例
     * @param {number} size - 图集尺寸（默认 2048x2048）
     * @param {number} glyphSize - 每个字形的像素尺寸（默认 64x64）
     */
    constructor(device, size = 2048, glyphSize = 64) {
        this.device = device;
        this.size = size;
        this.glyphSize = glyphSize;
        this.glyphsPerRow = Math.floor(size / glyphSize); // 每行可容纳的字形数

        // 字形缓存：key = `${fontId}-${char}`, value = { u, v }
        this.glyphMap = new Map();
        // 下一个空闲槽位
        this.nextSlot = 0;
        // 最大槽位数
        this.maxSlots = this.glyphsPerRow * this.glyphsPerRow;

        // 后台 Canvas 2D 上下文（用于字形烘焙）
        this._initCanvas();

        // WebGPU 纹理和绑定组
        this.texture = null;
        this.bindGroup = null;

        // 已初始化的标志
        this._initialized = false;
    }

    /**
     * 初始化后台 Canvas 2D 上下文
     */
    _initCanvas() {
        // 创建一个离屏 Canvas 用于字形渲染
        this.offscreenCanvas = new OffscreenCanvas(this.size, this.size);
        this.ctx = this.offscreenCanvas.getContext('2d');

        // 设置字体样式（可以从外部配置）
        this.ctx.textBaseline = 'top';
        this.ctx.textAlign = 'left';
        this.ctx.fillStyle = '#ffffff';
    }

    /**
     * 初始化 WebGPU 纹理
     */
    async init() {
        if (this._initialized) return;

        // 创建 WebGPU 纹理
        this.texture = this.device.createTexture({
            label: '字形纹理图集',
            size: { width: this.size, height: this.size },
            format: 'rgba8unorm',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
            mipLevelCount: 1,
            sampleCount: 1,
        });

        // 创建采样器
        const sampler = this.device.createSampler({
            label: '字形纹理采样器',
            addressModeU: 'clamp-to-edge',
            addressModeV: 'clamp-to-edge',
            magFilter: 'linear',
            minFilter: 'linear',
        });

        // 创建绑定组
        this.bindGroup = this.device.createBindGroup({
            label: '字形纹理绑定组',
            layout: this._createBindGroupLayout(),
            entries: [
                {
                    binding: 0,
                    resource: this.texture.createView(),
                },
                {
                    binding: 1,
                    resource: sampler,
                },
            ],
        });

        this._initialized = true;
        console.log(`✅ 纹理图集初始化完成：${this.size}x${this.size}，每字形 ${this.glyphSize}px`);
    }

    /**
     * 创建绑定组布局
     */
    _createBindGroupLayout() {
        return this.device.createBindGroupLayout({
            label: '字形纹理绑定组布局',
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
    }

    /**
     * 烘焙单个字符到纹理图集
     *
     * @param {number} fontId - 字体 ID
     * @param {string} char - 单个字符
     * @returns {{ u: number, v: number }} UV 坐标（归一化到 0-1）
     */
    bakeGlyph(fontId, char) {
        if (char.length === 0) return { u: 0, v: 0 };

        const key = `${fontId}-${char}`;

        // 缓存命中，直接返回 UV
        if (this.glyphMap.has(key)) {
            return this.glyphMap.get(key);
        }

        // 检查是否还有空位
        if (this.nextSlot >= this.maxSlots) {
            console.warn('⚠️ 纹理图集已满，无法烘焙更多字符');
            return { u: 0, v: 0 };
        }

        // 计算当前槽位的行列位置
        const row = Math.floor(this.nextSlot / this.glyphsPerRow);
        const col = this.nextSlot % this.glyphsPerRow;
        const x = col * this.glyphSize;
        const y = row * this.glyphSize;

        // 在离屏 Canvas 上绘制字符
        this.ctx.clearRect(x, y, this.glyphSize, this.glyphSize);
        
        // 根据字体 ID 设置字体
        // 字体 0：中文（Noto Serif SC）
        // 字体 1：拉丁（Noto Sans）
        // 字体 2：日文（Noto Sans JP）
        const fontFamily = fontId === 0 ? 'Noto Serif SC, serif'
                        : fontId === 1 ? 'Noto Sans, sans-serif'
                        : 'Noto Sans JP, sans-serif';
        this.ctx.font = `${this.glyphSize * 0.7}px ${fontFamily}`;

        // 绘制字符
        const metrics = this.ctx.measureText(char);
        const textWidth = metrics.width;
        const textHeight = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
        
        // 居中绘制
        const offsetX = (this.glyphSize - textWidth) / 2;
        const offsetY = (this.glyphSize - textHeight) / 2;
        this.ctx.fillText(char, x + offsetX, y + offsetY);

        // 计算 UV 坐标（归一化到 0-1）
        const uv = {
            u: x / this.size,
            v: y / this.size,
            w: this.glyphSize / this.size,
            h: this.glyphSize / this.size,
        };

        // 存入缓存
        this.glyphMap.set(key, uv);
        this.nextSlot++;

        // 每烘焙 100 个字符，将 Canvas 内容上传到 GPU 纹理
        // 这是一种批量上传策略，平衡 CPU 和 GPU 之间的数据同步频率
        if (this.nextSlot % 100 === 0) {
            this._uploadToGPU();
        }

        return uv;
    }

    /**
     * 批量烘焙字符串中的所有字符
     *
     * @param {number} fontId - 字体 ID
     * @param {string} text - 要烘焙的文本
     */
    bakeText(fontId, text) {
        for (const char of text) {
            this.bakeGlyph(fontId, char);
        }
    }

    /**
     * 将 Canvas 内容上传到 WebGPU 纹理
     *
     * 根据搜索结果[1](@ref)的策略1“共享内存减少拷贝”，
     * 我们通过 writeTexture 将离屏 Canvas 的像素数据直接上传到 GPU 纹理，
     * 避免额外的内存拷贝。
     */
    _uploadToGPU() {
        if (!this.texture || !this._initialized) return;

        // 从离屏 Canvas 获取像素数据
        const imageData = this.ctx.getImageData(0, 0, this.size, this.size);
        const pixelData = new Uint8Array(imageData.data.buffer);

        // 上传到 WebGPU 纹理
        this.device.queue.writeTexture(
            {
                texture: this.texture,
                mipLevel: 0,
                origin: { x: 0, y: 0, z: 0 },
            },
            pixelData,
            {
                offset: 0,
                bytesPerRow: this.size * 4,
                rowsPerImage: this.size,
            },
            {
                width: this.size,
                height: this.size,
                depthOrArrayLayers: 1,
            }
        );
    }

    /**
     * 获取字符的 UV 坐标（用于着色器）
     *
     * @param {number} fontId - 字体 ID
     * @param {string} char - 字符
     * @returns {{ u: number, v: number, w: number, h: number }}
     */
    getUV(fontId, char) {
        return this.glyphMap.get(`${fontId}-${char}`) || { u: 0, v: 0, w: 0, h: 0 };
    }

    /**
     * 强制完成所有待上传的数据
     */
    flush() {
        this._uploadToGPU();
    }

    /**
     * 获取图集的填充率
     *
     * @returns {number} 0-1 的填充率
     */
    getFillRatio() {
        return this.nextSlot / this.maxSlots;
    }

    /**
     * 销毁资源
     */
    destroy() {
        if (this.texture) this.texture.destroy();
        this.glyphMap.clear();
        this.offscreenCanvas = null;
        this.ctx = null;
        console.log('✅ 纹理图集资源已释放');
    }
}